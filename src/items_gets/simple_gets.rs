use std::io::Error;
use std::path::PathBuf;
use std::sync::Arc;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use axum_extra::extract::CookieJar;
use sqlx::{PgPool, Row};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub struct SharedStateStruct {
	pub pool: PgPool
}

pub async fn read_file_to_string(buf: &PathBuf) -> Result<String, Error> {
	let mut file = File::open(buf).await?;
	
	let mut body = String::new();
	file.read_to_string(&mut body).await?;
	
	Ok(body)
}

pub async fn replace_in_html(body: String, tag: &str, val: &str) -> String {
	let pattern = format!("<!--{{{tag}}}-->");
	body.replace(&pattern, val)
}

pub async fn get_user(jar: &CookieJar, state: &Arc<SharedStateStruct>) -> Option<i32> {
	match jar.get("SECURITY-COOKIE") {
		Some(val) => {
			let val = val.value();
			let result = sqlx::query("SELECT user_id FROM web_page.cookies WHERE cookie = $1")
				.bind(val)
				.fetch_one(&state.pool)
				.await.unwrap();
			
			match result.try_get("user_id") {
				Ok(user_id) => Some(user_id),
				Err(_) => None
			}
		}
		None => None
	}
}

pub async fn check_permission(state: &Arc<SharedStateStruct>, user_id: i32, permission: &str) -> bool {
	let result = sqlx::query("SELECT user_id FROM web_page.user_permissions WHERE user_id = $1 AND permission = $2")
		.bind(user_id).bind(permission)
		.fetch_optional(&state.pool)
		.await.unwrap();
	
	result.is_some()
}

async fn add_log_out(page: String, user_id: Option<i32>) -> String {
	if user_id.is_some() {
		let auth = read_file_to_string(&PathBuf::from("templates/auth.html")).await.unwrap();
		return replace_in_html(page, "auth", auth.as_ref()).await
	}
	page
}

pub async fn home(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> (StatusCode, Html<String>) {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return fallback(jar, State(state)).await
	}
	(StatusCode::OK, Html(read_file_to_string(&PathBuf::from("templates/index.html")).await.unwrap()))
}

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
) -> (StatusCode, Html<String>) {
	let user_id = get_user(&jar, &state).await;
	match user_id {
		Some(user_id) => {
			if check_permission(&state, user_id, "REG").await {
				return (StatusCode::OK, Html(read_file_to_string(&PathBuf::from("templates/registration.html")).await.unwrap()))
			}
			fallback(jar, State(state)).await
		},
		None => fallback(jar, State(state)).await
	}
}

pub async fn fallback(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> (StatusCode, Html<String>) {
	let user_id = get_user(&jar, &state).await;
	let page = read_file_to_string(&PathBuf::from("templates/error.html")).await.unwrap();
	let page = add_log_out(page, user_id).await;
	let page = replace_in_html(page, "error", "Not found").await;
	(StatusCode::NOT_FOUND, Html(page))
}

pub async fn bad_request(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> (StatusCode, Html<String>) {
	let user_id = get_user(&jar, &state).await;
	let page = read_file_to_string(&PathBuf::from("templates/error.html")).await.unwrap();
	let page = add_log_out(page, user_id).await;
	let page = replace_in_html(page, "error", "Bad request").await;
	(StatusCode::BAD_REQUEST, Html(page))
}