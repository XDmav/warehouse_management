use axum::body::Body;
use axum::extract::Path;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::fs;

use crate::app_error::{AppError, AppResult};

fn etag_for(bytes: &[u8]) -> String {
	let digest = Sha256::digest(bytes);
	let hex = base16ct::lower::encode_string(&digest);
	format!("\"{}\"", &hex[..16])
}

fn serve_static(bytes: Vec<u8>, content_type: &str, req_headers: &HeaderMap) -> Response {
	let etag = etag_for(&bytes);
	
	if let Some(inm) = req_headers.get(header::IF_NONE_MATCH) && inm.to_str().ok() == Some(etag.as_str()) {
		return (
			StatusCode::NOT_MODIFIED,
			[
				(header::ETAG, etag),
				(header::CACHE_CONTROL, "no-cache".to_string()),
			],
		).into_response();
	}
	
	(
		[
			(header::CONTENT_TYPE, content_type.to_string()),
			(header::CACHE_CONTROL, "no-cache".to_string()),
			(header::ETAG, etag),
		],
		Body::from(bytes),
	).into_response()
}

pub async fn get_image(
	Path(name): Path<String>,
	headers: HeaderMap,
) -> AppResult<Response> {
	let sanitized_name = sanitize_filename::sanitize(&name);
	if sanitized_name.is_empty() || sanitized_name != name {
		return Err(AppError::BadRequest("invalid static path".into()));
	}
	
	let mut buf = PathBuf::from("static/images");
	buf.push(&sanitized_name);
	
	let bytes = fs::read(&buf).await.map_err(|_| AppError::NotFound)?;
	
	let content_type = mime_guess::from_path(&buf)
		.first_or_octet_stream()
		.essence_str()
		.to_string();
	
	Ok(serve_static(bytes, &content_type, &headers))
}

pub async fn get_style(
	Path(name): Path<String>,
	headers: HeaderMap,
) -> AppResult<Response> {
	let sanitized_name = sanitize_filename::sanitize(&name);
	if sanitized_name.is_empty() || sanitized_name != name {
		return Err(AppError::BadRequest("invalid static path".into()));
	}
	
	let mut buf = PathBuf::from("static/css/dist");
	buf.push(&name);
	
	let bytes = fs::read(&buf).await.map_err(|_| AppError::NotFound)?;
	
	Ok(serve_static(bytes, "text/css", &headers))
}

pub async fn get_script(
	Path(name): Path<String>,
	headers: HeaderMap,
) -> AppResult<Response> {
	let sanitized_name = sanitize_filename::sanitize(&name);
	if sanitized_name.is_empty() || sanitized_name != name {
		return Err(AppError::BadRequest("invalid static path".into()));
	}
	
	let mut buf = PathBuf::from("static/js");
	buf.push(&name);
	
	let bytes = fs::read(&buf).await.map_err(|_| AppError::NotFound)?;
	
	Ok(serve_static(bytes, "text/javascript", &headers))
}