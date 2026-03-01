mod items_gets;
mod auth;

use axum::{http::StatusCode, routing::get, {serve, Router}};
use sqlx::{postgres::PgPoolOptions};
use std::{sync::Arc, time::Duration};
use tokio::{fs::File, io::AsyncReadExt, net::TcpListener};
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, timeout::TimeoutLayer, trace::TraceLayer};
use tracing::Level;
use tracing_subscriber::fmt;

use crate::items_gets::simple_gets::{fallback, home, login, registration, SharedStateStruct};
use crate::items_gets::{get_image, get_script, get_style};
use crate::auth::{logout, post_login, post_registration};

#[tokio::main]
async fn main() {
    fmt().with_max_level(Level::TRACE).init();
    
    let mut text = String::new();
    File::open("Secret")
        .await
        .unwrap()
        .read_to_string(&mut text)
        .await
        .unwrap();
    let text_vec: Vec<&str> = text.split("\n").collect();
    
    let url = text_vec[0];
    
    let pool = PgPoolOptions::new()
        .max_connections(32)
        .min_connections(2)
        .idle_timeout(Duration::new(60, 0))
        .connect(url.as_ref()).await.unwrap();
    let shared_state = Arc::new(SharedStateStruct{pool});
    
    let app = Router::new()
        .route("/", get(home))
        .route("/static/images/{name}", get(get_image))
        .route("/static/css/dist/{name}", get(get_style))
        .route("/static/js/{name}", get(get_script))
        .route("/login", get(login).post(post_login))
        .route("/registration", get(registration).post(post_registration))
        .route("/logout", get(logout))
        .fallback(fallback)
        .with_state(shared_state)
        .layer(
            ServiceBuilder::new()
                .concurrency_limit(10)
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::new(10, 0)))
                .layer(CompressionLayer::new())
        );
    
    let listen_adr = text_vec[1];
    let listener = TcpListener::bind(listen_adr).await.unwrap();
    serve(listener, app).await.unwrap();
}
