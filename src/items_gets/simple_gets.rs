use std::io::Error;
use std::path::PathBuf;
use std::sync::Arc;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use axum_extra::extract::CookieJar;
use sqlx::PgPool;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub struct SharedStateStruct {
	pub pool: PgPool
}

pub async fn get_file(path: &PathBuf) -> Result<File, Error> {
	File::open(&path).await
}

pub async fn read_file_to_string(buf: &PathBuf) -> Result<String, Error> {
	let mut file = get_file(buf).await?;
	
	let mut body = String::new();
	file.read_to_string(&mut body).await?;
	
	Ok(body)
}

pub async fn replace_in_html(body: String, tag: &str, val: &str) -> String {
	let pattern = format!("<!--{{{tag}}}-->");
	body.replace(&pattern, val)
}

pub async fn is_log_in(jar: &CookieJar, state: &Arc<SharedStateStruct>) -> bool {
	match jar.get("SECURITY-COOKIE") {
		Some(val) => {
			let val = val.value();
			let result = sqlx::query("SELECT user_id FROM web_page.cookies WHERE cookie = $1")
				.bind(val)
				.fetch_optional(&state.pool)
				.await.unwrap();
			
			match result {
				Some(_) => true,
				None => false
			}
		}
		None => false
	}
}

async fn add_log_out(page: String, is_log_in: bool) -> String {
	if is_log_in {
		let auth = read_file_to_string(&PathBuf::from("templates/auth.html")).await.unwrap();
		return replace_in_html(page, "auth", auth.as_ref()).await
	}
	page
}

pub async fn home(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> (StatusCode, Html<String>) {
	let is_log_in = is_log_in(&jar, &state).await;
	if !is_log_in {
		return fallback(jar, State(state)).await
	}
	(StatusCode::OK, Html(read_file_to_string(&PathBuf::from("templates/index.html")).await.unwrap()))
}

pub async fn login(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let is_log_in = is_log_in(&jar, &state).await;
	if is_log_in {
		return Err(Redirect::to("/"))
	}
	Ok(Html(read_file_to_string(&PathBuf::from("templates/login.html")).await.unwrap()))
}

pub async fn registration(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> (StatusCode, Html<String>) {
	let is_log_in = is_log_in(&jar, &state).await;
	if !is_log_in {
		return fallback(jar, State(state)).await
	}
	(StatusCode::OK, Html(read_file_to_string(&PathBuf::from("templates/registration.html")).await.unwrap()))
}

pub async fn fallback(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> (StatusCode, Html<String>) {
	let is_log_in = is_log_in(&jar, &state).await;
	let page = read_file_to_string(&PathBuf::from("templates/error.html")).await.unwrap();
	let page = add_log_out(page, is_log_in).await;
	let page = replace_in_html(page, "error", "Not found").await;
	(StatusCode::NOT_FOUND, Html(page))
}

pub async fn bad_request(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> (StatusCode, Html<String>) {
	let is_log_in = is_log_in(&jar, &state).await;
	let page = read_file_to_string(&PathBuf::from("templates/error.html")).await.unwrap();
	let page = add_log_out(page, is_log_in).await;
	let page = replace_in_html(page, "error", "Bad request").await;
	(StatusCode::BAD_REQUEST, Html(page))
}