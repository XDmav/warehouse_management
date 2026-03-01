use std::io::Error;
use std::path::PathBuf;
use std::sync::Arc;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;
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
	let pattern = format!("{{{{ {tag} }}}}");
	body.replace(&pattern, val)
}

async fn get_final_html(
	file_name: &PathBuf,
	jar: CookieJar,
	state: Arc<SharedStateStruct>
) -> String {
	let main_body = read_file_to_string(file_name).await.unwrap();
	let auth = match jar.get("SECURITY-COOKIE") {
		Some(val) => {
			let val = val.value();
			let result = sqlx::query("SELECT user_id FROM web_page.cookies WHERE cookie = $1")
				.bind(val)
				.fetch_optional(&state.pool)
				.await.unwrap();
			
			match result {
				Some(_) => read_file_to_string(&PathBuf::from("templates/auth_status/auth.html")).await.unwrap(),
				None => read_file_to_string(&PathBuf::from("templates/auth_status/not_auth.html")).await.unwrap()
			}
		}
		None => read_file_to_string(&PathBuf::from("templates/auth_status/not_auth.html")).await.unwrap()
	};
	replace_in_html(main_body, "auth", auth.as_str()).await
}

pub async fn home(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> Html<String> {
	Html(get_final_html(&PathBuf::from("templates/index.html"), jar, state).await)
}

pub async fn login(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> Html<String> {
	Html(get_final_html(&PathBuf::from("templates/login.html"), jar, state).await)
}

pub async fn registration(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> Html<String> {
	Html(get_final_html(&PathBuf::from("templates/registration.html"), jar, state).await)
}

pub async fn fallback(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> (StatusCode, Html<String>) {
	let page = get_final_html(&PathBuf::from("templates/error.html"), jar, state).await;
	let page = replace_in_html(page, "error", "Not found").await;
	(StatusCode::NOT_FOUND, Html(page))
}

pub async fn bad_request(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> (StatusCode, Html<String>) {
	let page = get_final_html(&PathBuf::from("templates/error.html"), jar, state).await;
	let page = replace_in_html(page, "error", "Bad request").await;
	(StatusCode::BAD_REQUEST, Html(page))
}