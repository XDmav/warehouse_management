use std::path::PathBuf;
use std::sync::Arc;
use axum::extract::State;
use axum::response::{Html, IntoResponse, Redirect};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use time::OffsetDateTime;

use crate::pages_gets::errors::fallback;
use crate::useful_funcs::{check_permission, get_user, read_file_to_string, SharedStateStruct};

pub mod static_gets;
pub mod errors;

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
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	match user_id {
		Some(user_id) => {
			if check_permission(&state, user_id, "REG").await {
				return Ok(Html(read_file_to_string(&PathBuf::from("templates/registration.html")).await.unwrap()))
			}
			Err(fallback(jar, State(state)).await.into_response())
		},
		None => Err(Redirect::to("/login").into_response())
	}
}

pub async fn logout(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	match jar.get("SECURITY-COOKIE") {
		Some(val) => {
			let val = val.value();
			let _ = sqlx::query("DELETE FROM web_page.cookies WHERE cookie = $1")
				.bind(val)
				.execute(&state.pool)
				.await;
		}
		_ => {}
	};
	
	let mut cookie = Cookie::new("SECURITY-COOKIE", "");
	cookie.set_secure(true);
	cookie.set_expires(OffsetDateTime::UNIX_EPOCH);
	(jar.add(cookie), Redirect::to("/"))
}

pub async fn home(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Err(Redirect::to("/login"))
	}
	Ok(Html(read_file_to_string(&PathBuf::from("templates/index.html")).await.unwrap()))
}

pub async fn stats(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Err(Redirect::to("/login"))
	}
	
	let page = read_file_to_string(&PathBuf::from("templates/stats.html")).await.unwrap();
	
	Ok(Html(page))
}