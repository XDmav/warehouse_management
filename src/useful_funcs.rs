use axum_extra::extract::CookieJar;
use sqlx::{PgPool, Row};
use std::io::Error;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use html_escape::encode_safe;

pub struct SharedStateStruct {
	pub pool: PgPool,
}

pub async fn read_file_to_string(buf: &PathBuf) -> Result<String, Error> {
	let mut file = File::open(buf).await?;
	
	let mut body = String::new();
	file.read_to_string(&mut body).await?;
	
	Ok(body)
}

pub async fn replace_text_in_html(body: String, tag: &str, val: &str) -> String {
	let pattern = format!("<!--{{{tag}}}-->");
	let safe = encode_safe(val);
	body.replace(&pattern, safe.as_ref())
}

pub async fn replace_html_in_html(body: String, tag: &str, val: &str) -> String {
	let pattern = format!("<!--{{{tag}}}-->");
	body.replace(&pattern, val)
}

pub async fn get_user(jar: &CookieJar, state: &Arc<SharedStateStruct>) -> Option<i32> {
	let cookie = jar.get("SECURITY-COOKIE")?.value();
	
	let result = sqlx::query("SELECT user_id FROM web_page.cookies WHERE cookie = $1")
		.bind(cookie)
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
		"SELECT user_id FROM web_page.user_permissions WHERE user_id = $1 AND permission = $2",
	)
		.bind(user_id)
		.bind(permission)
		.fetch_optional(&state.pool)
		.await
		.ok()
		.flatten()
		.is_some()
}

pub async fn add_log_out(page: String, user_id: Option<i32>) -> String {
	if user_id.is_some() {
		let auth = read_file_to_string(&PathBuf::from("templates/auth.html"))
			.await
			.unwrap();
		return replace_html_in_html(page, "auth", auth.as_ref()).await;
	}
	page
}
