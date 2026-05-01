use std::path::PathBuf;
use std::sync::Arc;

use sqlx::PgPool;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::app_error::{AppError, AppResult};

pub struct SharedStateStruct {
	pub pool: PgPool,
	pub templates: Templates,
}

pub struct Templates {
	pub index: Arc<str>,
	pub stats: Arc<str>,
	pub stats_sales: Arc<str>,
	pub stats_goods: Arc<str>,
	pub stats_warehouse: Arc<str>,
	pub receipt_create: Arc<str>,
	pub login: Arc<str>,
	pub registration: Arc<str>,
	pub auth: Arc<str>,
	pub error: Arc<str>,
}

impl Templates {
	pub async fn load() -> AppResult<Self> {
		async fn load_one(path: &str) -> AppResult<Arc<str>> {
			let mut file = File::open(path).await
				.map_err(|e| AppError::Internal(format!("template '{path}': {e}")))?;
			let mut buf = String::new();
			file.read_to_string(&mut buf).await
				.map_err(|e| AppError::Internal(format!("template '{path}': {e}")))?;
			Ok(Arc::from(buf))
		}
		
		let (
			index, stats, stats_sales, stats_goods, stats_warehouse,
			receipt_create, login, registration, auth, error,
		) = tokio::try_join!(
            load_one("templates/index.html"),
            load_one("templates/stats.html"),
            load_one("templates/stats_sales.html"),
            load_one("templates/stats_goods.html"),
            load_one("templates/stats_warehouse.html"),
            load_one("templates/receipt_create.html"),
            load_one("templates/login.html"),
            load_one("templates/registration.html"),
            load_one("templates/auth.html"),
            load_one("templates/error.html"),
        )?;
		
		Ok(Self {
			index, stats, stats_sales, stats_goods, stats_warehouse,
			receipt_create, login, registration, auth, error,
		})
	}
}

pub fn hash_cookie(cookie: &str) -> String {
	use sha2::{Digest, Sha256};
	let digest = Sha256::digest(cookie.as_bytes());
	base16ct::lower::encode_string(&digest)
}

pub async fn read_file_to_string(buf: &PathBuf) -> AppResult<String> {
	let mut file = File::open(buf).await?;
	let mut body = String::new();
	file.read_to_string(&mut body).await?;
	Ok(body)
}

pub fn replace_text_in_html(body: String, tag: &str, val: &str) -> String {
	let pattern = format!("<!--{{{tag}}}-->");
	let safe = html_escape::encode_safe(val);
	body.replace(&pattern, safe.as_ref())
}

pub fn replace_html_in_html(body: String, tag: &str, val: &str) -> String {
	let pattern = format!("<!--{{{tag}}}-->");
	body.replace(&pattern, val)
}

pub async fn get_user(
	jar: &axum_extra::extract::CookieJar,
	state: &Arc<SharedStateStruct>,
) -> Option<i32> {
	use sqlx::Row;
	let raw = jar.get("SECURITY-COOKIE")?.value();
	let hash = hash_cookie(raw);
	
	let row = sqlx::query(
		"SELECT user_id FROM web_page.cookies \
         WHERE cookie_hash = $1 AND expires_at > now()"
	)
		.bind(&hash)
		.fetch_optional(&state.pool)
		.await
		.ok()??;
	
	row.try_get("user_id").ok()
}

pub async fn check_permission(
	state: &Arc<SharedStateStruct>,
	user_id: i32,
	permission: &str,
) -> bool {
	sqlx::query(
		"SELECT user_id FROM web_page.users_permissions \
         WHERE user_id = $1 AND permission = $2"
	)
		.bind(user_id)
		.bind(permission)
		.fetch_optional(&state.pool)
		.await
		.ok()
		.flatten()
		.is_some()
}

pub fn add_log_out(page: String, user_id: Option<i32>, templates: &Templates) -> String {
	if user_id.is_some() {
		replace_html_in_html(page, "auth", &templates.auth)
	} else {
		page
	}
}