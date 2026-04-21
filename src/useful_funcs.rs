use axum_extra::extract::CookieJar;
use sqlx::{PgPool, Row};
use std::io::Error;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use html_escape::encode_safe;
use sha2::{Digest, Sha256};
use crate::app_error::AppResult;

pub struct SharedStateStruct {
	pub pool: PgPool,
}

pub async fn read_file_to_string(buf: &PathBuf) -> Result<String, Error> {
	let mut file = File::open(buf).await?;
	
	let mut body = String::new();
	file.read_to_string(&mut body).await?;
	
	Ok(body)
}

pub fn replace_text_in_html(body: String, tag: &str, val: &str) -> String {
	let pattern = format!("<!--{{{tag}}}-->");
	let safe = encode_safe(val);
	body.replace(&pattern, safe.as_ref())
}

pub fn replace_html_in_html(body: String, tag: &str, val: &str) -> String {
	let pattern = format!("<!--{{{tag}}}-->");
	body.replace(&pattern, val)
}

pub fn hash_cookie(cookie: &str) -> String {
	let digest = Sha256::digest(cookie.as_bytes());
	base16ct::lower::encode_string(&digest)
}

pub async fn get_user(jar: &CookieJar, state: &Arc<SharedStateStruct>) -> Option<i32> {
	let raw = jar.get("SECURITY-COOKIE")?.value();
	let hash = hash_cookie(raw);
	
	let result = sqlx::query(
		"SELECT user_id FROM web_page.cookies
         WHERE cookie_hash = $1 AND expires_at > now()"
	)
		.bind(&hash)
		.fetch_optional(&state.pool)
		.await
		.ok()??;
	
	result.try_get("user_id").ok()
}

pub async fn check_permission(
	state: &Arc<SharedStateStruct>,
	user_id: i32,
	permission: &str,
) -> bool {
	sqlx::query(
		"SELECT user_id FROM web_page.users_permissions WHERE user_id = $1 AND permission = $2",
	)
		.bind(user_id)
		.bind(permission)
		.fetch_optional(&state.pool)
		.await
		.ok()
		.flatten()
		.is_some()
}

pub async fn add_log_out(page: String, user_id: Option<i32>) -> AppResult<String> {
	if user_id.is_some() {
		let auth = read_file_to_string(&PathBuf::from("templates/auth.html")).await?;
		return Ok(replace_html_in_html(page, "auth", &auth));
	}
	Ok(page)
}
