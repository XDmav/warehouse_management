use argon2::{password_hash::PasswordHasher, Argon2, PasswordHash, PasswordVerifier};
use axum::extract::State;
use axum::response::{IntoResponse, Redirect};
use axum::Form;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use base16ct::lower;
use email_address::EmailAddress;
use rand::rngs::StdRng;
use rand::Rng;
use serde::Deserialize;
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;
use axum::http::StatusCode;
use time::OffsetDateTime;

use crate::pages_gets::{login, registration};
use crate::useful_funcs::{check_permission, get_user, SharedStateStruct};

#[derive(Deserialize)]
pub struct UserInfo {
	email: String,
	password: String,
}

pub async fn post_login(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Form(payload): Form<UserInfo>,
) -> impl IntoResponse {
	let account = sqlx::query("SELECT user_id, password_hash FROM web_page.users WHERE email = $1")
		.bind(&payload.email)
		.fetch_optional(&state.pool)
		.await
		.unwrap();
	
	let account = match account {
		Some(account) => account,
		None => return Err(login(jar, State(state)).await),
	};
	
	let hash = account.get("password_hash");
	
	let parsed_hash = PasswordHash::new(hash).unwrap();
	
	if Argon2::default()
		.verify_password(payload.password.as_bytes(), &parsed_hash)
		.is_err()
	{
		return Err(login(jar, State(state)).await);
	}
	
	let mut buf = [0; 64];
	let mut rng: StdRng = rand::make_rng();
	rng.fill_bytes(&mut buf);
	
	let cookie = lower::encode_string(&buf);
	
	let id: i32 = account.get("user_id");
	
	sqlx::query("INSERT INTO web_page.cookies (cookie, user_id) VALUES ($1, $2)")
		.bind(&cookie)
		.bind(id)
		.execute(&state.pool)
		.await
		.unwrap();
	
	let mut cookie = Cookie::new("SECURITY-COOKIE", cookie);
	cookie.set_secure(true);
	cookie.set_http_only(true);
	cookie.set_same_site(SameSite::Lax);
	cookie.set_path("/");
	
	let mut now = OffsetDateTime::now_utc();
	now += Duration::new(60 * 60 * 24 * 30, 0);
	
	cookie.set_expires(now);
	
	Ok((jar.add(cookie), Redirect::to("/")))
}

pub async fn logout(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> impl IntoResponse {
	if let Some(val) = jar.get("SECURITY-COOKIE") {
		let val = val.value();
		let _ = sqlx::query("DELETE FROM web_page.cookies WHERE cookie = $1")
			.bind(val)
			.execute(&state.pool)
			.await;
	};
	
	let mut cookie = Cookie::new("SECURITY-COOKIE", "");
	cookie.set_secure(true);
	cookie.set_http_only(true);
	cookie.set_same_site(SameSite::Lax);
	cookie.set_path("/");
	cookie.set_expires(OffsetDateTime::UNIX_EPOCH);
	(jar.add(cookie), Redirect::to("/login"))
}

pub async fn post_registration(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Form(payload): Form<UserInfo>,
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	match user_id {
		Some(user_id) => {
			if !check_permission(&state, user_id, "REG").await {
				return Err((StatusCode::UNAUTHORIZED, "Unauthorized").into_response());
			}
		}
		None => return Err((StatusCode::UNAUTHORIZED, "Unauthorized").into_response()),
	}
	
	if !EmailAddress::is_valid(&payload.email) {
		return Err(registration(jar, State(state)).await.into_response());
	}
	
	let account = sqlx::query("SELECT user_id FROM web_page.users WHERE email = $1")
		.bind(&payload.email)
		.fetch_optional(&state.pool)
		.await
		.unwrap();
	
	if account.is_some() {
		return Err(registration(jar, State(state)).await.into_response());
	}
	
	let password_hash = Argon2::default()
		.hash_password(payload.password.as_bytes())
		.unwrap()
		.to_string();
	
	let result = sqlx::query("INSERT INTO web_page.users(email, password_hash) VALUES ($1, $2)")
		.bind(&payload.email)
		.bind(password_hash)
		.execute(&state.pool)
		.await;
	
	match result {
		Ok(_) => Ok(registration(jar, State(state)).await),
		Err(_) => Err(registration(jar, State(state)).await.into_response()),
	}
}
