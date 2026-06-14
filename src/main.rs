mod auth_posts;
mod pages_gets;
mod pages_posts;
mod useful_funcs;
mod api;
pub mod csrf;
pub mod app_error;

use axum::{http::StatusCode, middleware, routing::get, {serve, Router}};
use sqlx::postgres::PgPoolOptions;
use std::{sync::Arc, time::Duration};
use std::time::Instant;
use axum::routing::post;
use tokio::{fs::File, io::AsyncReadExt, net::TcpListener};
use tower::ServiceBuilder;
use tower_governor::governor::GovernorConfigBuilder;
use tower_http::{compression::CompressionLayer, timeout::TimeoutLayer, trace::TraceLayer};
use tracing::Level;
use tracing_subscriber::fmt;

use auth_posts::{post_login, post_registration};
use pages_gets::errors::not_found;
use pages_gets::static_gets::{get_image, get_script, get_style};
use pages_gets::{
	create_receipt_page, home, login, registration, stats, stats_goods, stats_sales,
	stats_warehouse,
};
use pages_posts::create_receipt;
use useful_funcs::SharedStateStruct;
use api::goods_stock;
use tower_governor::{GovernorLayer};
use crate::api::{goods_discounts, goods_list};
use crate::auth_posts::logout;

use tokio::signal;
use tokio::time::{interval, timeout, MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use crate::useful_funcs::Templates;

async fn shutdown_signal() {
	let ctrl_c = async {
		signal::ctrl_c()
			.await
			.expect("failed to install Ctrl+C handler");
	};
	
	#[cfg(unix)]
	let terminate = async {
		signal::unix::signal(signal::unix::SignalKind::terminate())
			.expect("failed to install SIGTERM handler")
			.recv()
			.await;
	};
	
	#[cfg(not(unix))]
	let terminate = std::future::pending::<()>();
	
	tokio::select! {
        _ = ctrl_c    => tracing::info!("Получен Ctrl+C, начинаем graceful shutdown"),
        _ = terminate => tracing::info!("Получен SIGTERM, начинаем graceful shutdown"),
    }
}

#[tokio::main]
async fn main() {
	fmt().with_max_level(Level::TRACE).init();
	
	let mut text = String::new();
	File::open("Secret").await
		.unwrap()
		.read_to_string(&mut text)
		.await
		.unwrap();
	
	let mut lines = text.lines().map(str::trim).filter(|s| !s.is_empty());
	
	let url = lines.next().expect("DB url missing");
	let listen_addr = lines.next().expect("listen addr missing");
	
	let pool = PgPoolOptions::new()
		.max_connections(96)
		.min_connections(2)
		.acquire_timeout(Duration::from_secs(5))
		.idle_timeout(Duration::from_secs(60))
		.connect(url)
		.await
		.unwrap();
	
	sqlx::query("REFRESH MATERIALIZED VIEW v_abc_analysis")
		.execute(&pool)
		.await
		.map_err(|e| tracing::warn!("Первичный refresh ABC не удался: {}", e))
		.ok();
	
	let shutdown_token = CancellationToken::new();
	
	let cleanup_handle = tokio::spawn({
		let pool = pool.clone();
		let token = shutdown_token.clone();
		async move {
			let mut interval = interval(Duration::from_secs(3600));
			interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
			interval.tick().await;
			
			loop {
				tokio::select! {
                _ = interval.tick() => {
                    let result = sqlx::query(
                        "DELETE FROM web_page.cookies WHERE expires_at < now()"
                    )
                    .execute(&pool)
                    .await;

                    match result {
                        Ok(r) => tracing::debug!(
                            "Удалено просроченных cookie: {}", r.rows_affected()
                        ),
                        Err(e) => tracing::warn!("Ошибка очистки cookie: {}", e),
                    }
                }
                _ = token.cancelled() => {
                    tracing::info!("Cleanup task: получен shutdown, завершаемся");
                    break;
                }
            }
			}
		}
	});
	
	let refresh_handle = tokio::spawn({
		let pool = pool.clone();
		let token = shutdown_token.clone();
		async move {
			let mut interval = interval(Duration::from_secs(300));
			interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
			interval.tick().await;
			
			loop {
				tokio::select! {
                _ = interval.tick() => {
                    let start = Instant::now();
                    let result = sqlx::query(
                        "REFRESH MATERIALIZED VIEW CONCURRENTLY v_abc_analysis"
                    )
                    .execute(&pool)
                    .await;

                    match result {
                        Ok(_) => tracing::info!(
                            "ABC refresh выполнен за {:?}",
                            start.elapsed()
                        ),
                        Err(e) => tracing::warn!("ABC refresh не удался: {}", e),
                    }
                }
                _ = token.cancelled() => {
                    tracing::info!("ABC refresh task: shutdown");
                    break;
                }
            }
			}
		}
	});
	
	let templates = Templates::load()
		.await
		.expect("Не удалось загрузить шаблоны при старте сервера");
	
	let shared_state = Arc::new(SharedStateStruct {
		pool: pool.clone(),
		templates,
	});
	
	let login_governor = Arc::new(
		GovernorConfigBuilder::default()
			.per_second(1)        // не чаще раза в 1 сек
			.burst_size(5)        // пакет до 5
			.key_extractor(tower_governor::key_extractor::SmartIpKeyExtractor)
			.finish()
			.unwrap()
	);
	
	let app = Router::new()
		.route("/", get(home))
		
		.route("/stats", get(stats))
		.route("/stats/sales", get(stats_sales))
		.route("/stats/goods", get(stats_goods))
		.route("/stats/warehouse", get(stats_warehouse))
		
		.route("/receipts", get(create_receipt_page).post(create_receipt))
		.route("/receipts/new", get(create_receipt_page).post(create_receipt))
		
		.route("/api/goods/{goods_id}/stock", get(goods_stock))
		.route("/api/goods", get(goods_list))
		.route("/api/discounts/{card_number}", get(goods_discounts))
		
		.route("/static/images/{*name}", get(get_image))
		.route("/static/css/dist/{*name}", get(get_style))
		.route("/static/js/{*name}", get(get_script))
		
		.route("/login", get(login).post(post_login)
			.layer(GovernorLayer::new(login_governor.clone())))
		.route("/registration", get(registration).post(post_registration)
			.layer(GovernorLayer::new(login_governor)))
		.route("/logout", post(logout))
		
		.fallback(not_found)
		.with_state(shared_state)
		
		.layer(middleware::from_fn(csrf::csrf_middleware))
		.layer(
			ServiceBuilder::new()
				.concurrency_limit(32)
				.layer(TraceLayer::new_for_http())
				.layer(TimeoutLayer::with_status_code(
					StatusCode::REQUEST_TIMEOUT,
					Duration::new(5, 0),
				))
				.layer(CompressionLayer::new())
		);
	
	let listener = TcpListener::bind(listen_addr).await.unwrap();
	tracing::info!("Listening on {}", listen_addr);
	
	let server_result = serve(
		listener,
		app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
	)
		.with_graceful_shutdown({
			let token = shutdown_token.clone();
			async move {
				shutdown_signal().await;
				tracing::info!("Инициируем graceful shutdown");
				token.cancel();
			}
		})
		.await;
	
	if let Err(e) = server_result {
		tracing::error!("Server Error: {}", e);
	}
	
	match timeout(Duration::from_secs(10), cleanup_handle).await {
		Ok(Ok(())) => tracing::info!("Cleanup task ended"),
		Ok(Err(e)) => tracing::warn!("Cleanup task panicked: {}", e),
		Err(_) => tracing::warn!("Cleanup task did not complete within 10 seconds"),
	}
	
	match timeout(Duration::from_secs(10), refresh_handle).await {
		Ok(Ok(())) => tracing::info!("ABC refresh task ended"),
		Ok(Err(e)) => tracing::warn!("ABC refresh task panicked: {}", e),
		Err(_) => tracing::warn!("ABC refresh task did not complete within 10 seconds"),
	}
	
	pool.close().await;
	
	tracing::info!("Server stopped");
}
