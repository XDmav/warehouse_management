use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum_extra::extract::CookieJar;
use std::path::PathBuf;
use std::sync::Arc;

use crate::useful_funcs::{
	add_log_out, get_user, read_file_to_string, replace_html_in_html, SharedStateStruct,
};

pub async fn not_found(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	let page = read_file_to_string(&PathBuf::from("templates/error.html"))
		.await
		.unwrap();
	let page = add_log_out(page, user_id).await;
	let page = replace_html_in_html(page, "error", "Not found").await;
	(StatusCode::NOT_FOUND, Html(page))
}

pub async fn bad_request(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	let page = read_file_to_string(&PathBuf::from("templates/error.html"))
		.await
		.unwrap();
	let page = add_log_out(page, user_id).await;
	let page = replace_html_in_html(page, "error", "Bad request").await;
	(StatusCode::BAD_REQUEST, Html(page))
}

pub async fn server_error(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	let page = read_file_to_string(&PathBuf::from("templates/error.html"))
		.await
		.unwrap();
	let page = add_log_out(page, user_id).await;
	let page = replace_html_in_html(page, "error", "Internal server error").await;
	(StatusCode::INTERNAL_SERVER_ERROR, Html(page))
}

pub async fn unauthorized(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	let page = read_file_to_string(&PathBuf::from("templates/error.html"))
		.await
		.unwrap();
	let page = add_log_out(page, user_id).await;
	let page = replace_html_in_html(page, "error", "Unauthorized").await;
	(StatusCode::UNAUTHORIZED, Html(page))
}