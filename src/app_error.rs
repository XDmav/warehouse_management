use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
	Db(sqlx::Error),
	Io(std::io::Error),
	PasswordHash(argon2::password_hash::Error),
	NotFound,
	BadRequest(String),
	Unauthorized,
	Internal(String),
}

impl fmt::Display for AppError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			AppError::Db(e) => write!(f, "DB error: {e}"),
			AppError::Io(e) => write!(f, "IO error: {e}"),
			AppError::PasswordHash(e) => write!(f, "Password hash error: {e}"),
			AppError::NotFound => write!(f, "Not found"),
			AppError::BadRequest(m) => write!(f, "Bad request: {m}"),
			AppError::Unauthorized => write!(f, "Unauthorized"),
			AppError::Internal(m) => write!(f, "Internal: {m}"),
		}
	}
}

impl From<sqlx::Error> for AppError {
	fn from(e: sqlx::Error) -> Self { AppError::Db(e) }
}
impl From<std::io::Error> for AppError {
	fn from(e: std::io::Error) -> Self { AppError::Io(e) }
}
impl From<argon2::password_hash::Error> for AppError {
	fn from(e: argon2::password_hash::Error) -> Self { AppError::PasswordHash(e) }
}

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		tracing::error!("{}", self);
		
		let (status, msg): (StatusCode, &str) = match self {
			AppError::NotFound      => (StatusCode::NOT_FOUND, "Not found"),
			AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "Bad request"),
			AppError::Unauthorized  => (StatusCode::UNAUTHORIZED, "Unauthorized"),
			AppError::Db(_)
			| AppError::Io(_)
			| AppError::PasswordHash(_)
			| AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
		};
		
		(status, msg.to_string()).into_response()
	}
}

pub type AppResult<T> = Result<T, AppError>;