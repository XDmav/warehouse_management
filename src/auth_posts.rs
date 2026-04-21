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
use time::OffsetDateTime;
use crate::app_error::{AppError, AppResult};
use crate::useful_funcs::{check_permission, get_user, hash_cookie, SharedStateStruct};

const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$ZHGJerRF83khoMUb52Jo2g$s7nwb02r25ThJjCCWgSEPmoWJVOUe3eD3QdfkBnitRM";

#[derive(Deserialize)]
pub struct UserInfo {
	email: String,
	password: String,
}

pub async fn post_login(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Form(payload): Form<UserInfo>,
) -> AppResult<impl IntoResponse> {
	let account = sqlx::query("SELECT user_id, password_hash FROM web_page.users WHERE email = $1")
		.bind(&payload.email)
		.fetch_optional(&state.pool)
		.await?;
	
	let hash_string: String = match &account {
		Some(acc) => acc.try_get("password_hash").map_err(|e| AppError::Internal(e.to_string()))?,
		None => DUMMY_HASH.to_string(),
	};
	
	let parsed_hash = PasswordHash::new(&hash_string)
		.map_err(|e| AppError::Internal(format!("bad stored hash: {e}")))?;
	
	let password_ok = Argon2::default()
		.verify_password(payload.password.as_bytes(), &parsed_hash)
		.is_ok();
	
	if account.is_none() || !password_ok {
		return Ok(Redirect::to("/login?error=invalid").into_response());
	}
	
	let account = account.unwrap();
	let user_id: i32 = account
		.try_get("user_id")
		.map_err(|e| AppError::Internal(format!("column 'user_id': {e}")))?;
	
	let mut buf = [0; 64];
	let mut rng: StdRng = rand::make_rng();
	rng.fill_bytes(&mut buf);
	
	let cookie = lower::encode_string(&buf);
	let cookie_hash = hash_cookie(&cookie);
	
	sqlx::query("INSERT INTO web_page.cookies (cookie_hash, user_id) VALUES ($1, $2)")
		.bind(&cookie_hash)
		.bind(user_id)
		.execute(&state.pool)
		.await?;
	
	let mut cookie = Cookie::new("SECURITY-COOKIE", cookie);
	cookie.set_secure(true);
	cookie.set_http_only(true);
	cookie.set_same_site(SameSite::Lax);
	cookie.set_path("/");
	
	let mut now = OffsetDateTime::now_utc();
	now += Duration::new(60 * 60 * 24 * 30, 0);
	
	cookie.set_expires(now);
	
	Ok((jar.add(cookie), Redirect::to("/")).into_response())
}

pub async fn logout(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> AppResult<impl IntoResponse> {
	if let Some(val) = jar.get("SECURITY-COOKIE") {
		let hash = hash_cookie(val.value());
		let _ = sqlx::query("DELETE FROM web_page.cookies WHERE cookie_hash = $1")
			.bind(&hash)
			.execute(&state.pool)
			.await;
	};
	
	let mut cookie = Cookie::new("SECURITY-COOKIE", "");
	cookie.set_secure(true);
	cookie.set_http_only(true);
	cookie.set_same_site(SameSite::Lax);
	cookie.set_path("/");
	cookie.set_expires(OffsetDateTime::UNIX_EPOCH);
	Ok((jar.add(cookie), Redirect::to("/login")))
}

pub async fn post_registration(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Form(payload): Form<UserInfo>,
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state)
		.await
		.ok_or(AppError::Unauthorized)?;
	
	if !check_permission(&state, user_id, "REG").await {
		return Err(AppError::Unauthorized);
	}
	
	if !EmailAddress::is_valid(&payload.email) {
		return Ok(Redirect::to("/registration?error=invalid_email").into_response());
	}
	
	let email = payload.email.trim().to_lowercase();
	
	let account = sqlx::query("SELECT user_id FROM web_page.users WHERE email = $1")
		.bind(&email)
		.fetch_optional(&state.pool)
		.await?;
	
	if account.is_some() {
		return Ok(Redirect::to("/registration?error=email_taken").into_response());
	}
	
	let password_hash = Argon2::default()
		.hash_password(payload.password.as_bytes())
		.map_err(|e| AppError::Internal(format!("argon2 hash: {e}")))?
		.to_string();
	
	let result = sqlx::query("INSERT INTO web_page.users(email, password_hash) VALUES ($1, $2)")
		.bind(&email)
		.bind(password_hash)
		.execute(&state.pool)
		.await;
	
	match result {
		Ok(_) => Ok(Redirect::to("/registration?success=1").into_response()),
		Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
			Ok(Redirect::to("/registration?error=email_taken").into_response())
		}
		Err(e) => Err(AppError::Db(e)),
	}
}
