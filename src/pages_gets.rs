use std::path::PathBuf;
use std::sync::Arc;
use axum::extract::State;
use axum::response::{Html, IntoResponse, Redirect};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use sqlx::Row;
use time::OffsetDateTime;

use crate::pages_gets::errors::fallback;
use crate::useful_funcs::{check_permission, get_user, read_file_to_string, replace_in_html, SharedStateStruct};

pub mod static_gets;
pub mod errors;

pub async fn login(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_some() {
		return Err(Redirect::to("/"))
	}
	Ok(Html(read_file_to_string(&PathBuf::from("templates/login.html")).await.unwrap()))
}

pub async fn registration(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	match user_id {
		Some(user_id) => {
			if check_permission(&state, user_id, "REG").await {
				return Ok(Html(read_file_to_string(&PathBuf::from("templates/registration.html")).await.unwrap()))
			}
			Err(fallback(jar, State(state)).await.into_response())
		},
		None => Err(Redirect::to("/login").into_response())
	}
}

pub async fn logout(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	match jar.get("SECURITY-COOKIE") {
		Some(val) => {
			let val = val.value();
			let _ = sqlx::query("DELETE FROM web_page.cookies WHERE cookie = $1")
				.bind(val)
				.execute(&state.pool)
				.await;
		}
		_ => {}
	};
	
	let mut cookie = Cookie::new("SECURITY-COOKIE", "");
	cookie.set_secure(true);
	cookie.set_expires(OffsetDateTime::UNIX_EPOCH);
	(jar.add(cookie), Redirect::to("/"))
}

pub async fn home(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Err(Redirect::to("/login"))
	}
	Ok(Html(read_file_to_string(&PathBuf::from("templates/index.html")).await.unwrap()))
}

pub async fn stats(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Err(Redirect::to("/login"))
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/stats.html"))
		.await
		.unwrap();
	
	let receipts: i64 = sqlx::query("SELECT COUNT(*) as count FROM receipts")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	let revenue: Option<f64> = sqlx::query("SELECT SUM(quantity*price*(1-discount/100.0)) as sum FROM receipt_items")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.try_get("sum")
		.ok();
	
	let goods: i64 = sqlx::query("SELECT COUNT(*) as count FROM goods")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	page = replace_in_html(page,"receipts",&receipts.to_string()).await;
	
	page = replace_in_html(page,"revenue",&format!("{:.2}", revenue.unwrap_or(0.0))).await;
	
	page = replace_in_html(page,"goods", &goods.to_string()).await;
	
	Ok(Html(page))
}

pub async fn stats_sales(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Err(Redirect::to("/login"))
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/stats_sales.html"))
		.await
		.unwrap();
	
	let rows = sqlx::query(
		"SELECT g.name, SUM(ri.quantity) as sold
         FROM receipt_items ri
         JOIN goods g ON g.goods_id = ri.goods_id
         GROUP BY g.goods_id
         ORDER BY sold DESC
         LIMIT 10"
	)
		.fetch_all(&state.pool)
		.await
		.unwrap();
	
	let mut list = String::new();
	
	for r in rows {
		let name: String = r.get("name");
		let sold: i64 = r.get("sold");
		
		list.push_str(
			&format!("<li>{} — {} шт.</li>", name, sold)
		);
	}
	
	page = replace_in_html(page,"top_goods",&list).await;
	
	Ok(Html(page))
}

pub async fn stats_goods(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Err(Redirect::to("/login"))
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/stats_goods.html"))
		.await
		.unwrap();
	
	let rows = sqlx::query(
		"SELECT g.name, SUM(ri.quantity*ri.price)::float8 as revenue
         FROM receipt_items ri
         JOIN goods g ON g.goods_id = ri.goods_id
         GROUP BY g.goods_id
         ORDER BY revenue DESC
         LIMIT 10"
	)
		.fetch_all(&state.pool)
		.await
		.unwrap();
	
	let mut list = String::new();
	
	for r in rows {
		let name: String = r.get("name");
		let revenue: f64 = r.get("revenue");
		
		list.push_str(
			&format!("<li>{} — {:.2}</li>", name, revenue)
		);
	}
	
	page = replace_in_html(page,"goods_revenue",&list).await;
	
	Ok(Html(page))
}

pub async fn stats_warehouse(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Err(Redirect::to("/login"))
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/stats_warehouse.html"))
		.await
		.unwrap();
	
	let receipts: i64 = sqlx::query(
		"SELECT COUNT(*) as count FROM goods_receipts"
	)
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	let writeoffs: i64 = sqlx::query(
		"SELECT COUNT(*) as count FROM writeoff_acts"
	)
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	page = replace_in_html(page,"receipts",&receipts.to_string()).await;
	
	page = replace_in_html(page,"writeoffs",&writeoffs.to_string()).await;
	
	Ok(Html(page))
}