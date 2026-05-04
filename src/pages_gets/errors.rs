use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum_extra::extract::CookieJar;

use crate::useful_funcs::{
	add_log_out, get_user, replace_html_in_html, SharedStateStruct,
};

async fn render_error_page(
	jar: &CookieJar,
	state: &Arc<SharedStateStruct>,
	status: StatusCode,
	message: &str,
) -> Response {
	let template = state.templates.error.to_string();
	let user_id = get_user(jar, state).await;
	let page = add_log_out(template, user_id, &state.templates);
	let page = replace_html_in_html(page, "error", message);
	(status, Html(page)).into_response()
}

pub async fn not_found(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> impl IntoResponse {
	render_error_page(&jar, &state, StatusCode::NOT_FOUND, "Not found").await
}

pub async fn _bad_request(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> impl IntoResponse {
	render_error_page(&jar, &state, StatusCode::BAD_REQUEST, "Bad request").await
}

pub async fn _server_error(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> impl IntoResponse {
	render_error_page(
		&jar, &state,
		StatusCode::INTERNAL_SERVER_ERROR,
		"Internal server error",
	).await
}

pub async fn unauthorized(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> impl IntoResponse {
	render_error_page(&jar, &state, StatusCode::UNAUTHORIZED, "Unauthorized").await
}