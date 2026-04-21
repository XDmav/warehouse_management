use std::path::PathBuf;
use std::sync::Arc;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum_extra::extract::CookieJar;

use crate::useful_funcs::{
	add_log_out, get_user, read_file_to_string, replace_html_in_html, SharedStateStruct,
};

async fn render_error_page(
	jar: &CookieJar,
	state: &Arc<SharedStateStruct>,
	status: StatusCode,
	message: &str,
) -> Response {
	let template = match read_file_to_string(&PathBuf::from("templates/error.html")).await {
		Ok(t) => t,
		Err(_) => {
			return (status, message.to_string()).into_response();
		}
	};
	
	let user_id = get_user(jar, state).await;
	
	let page = if user_id.is_some() {
		match add_log_out(template.clone(), user_id).await {
			Ok(p) => p,
			Err(_) => {
				template
			}
		}
	} else {
		template
	};
	
	let page = replace_html_in_html(page, "error", message);
	
	(status, Html(page)).into_response()
}

pub async fn not_found(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> impl IntoResponse {
	render_error_page(&jar, &state, StatusCode::NOT_FOUND, "Not found").await
}

pub async fn unauthorized(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> impl IntoResponse {
	render_error_page(&jar, &state, StatusCode::UNAUTHORIZED, "Unauthorized").await
}