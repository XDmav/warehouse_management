use axum::body::Body;
use axum::extract::Path;
use axum::http::header;
use axum::response::IntoResponse;
use std::path::PathBuf;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::app_error::{AppError, AppResult};
use crate::useful_funcs::read_file_to_string;

pub async fn get_image(
	Path(name): Path<String>,
) -> AppResult<impl IntoResponse> {
	let sanitized_name = sanitize_filename::sanitize(&name);
	if sanitized_name.is_empty() || sanitized_name != name {
		return Err(AppError::BadRequest("invalid static path".into()));
	}
	
	let mut buf = PathBuf::from("static/images");
	buf.push(&sanitized_name);
	
	let file = File::open(&buf).await.map_err(|_| AppError::NotFound)?;
	
	let content_type = mime_guess::from_path(&buf)
		.first_or_octet_stream()
		.essence_str()
		.to_string();
	
	let stream = ReaderStream::new(file);
	let body = Body::from_stream(stream);
	
	let headers = [
		(header::CONTENT_TYPE, content_type),
		(header::CACHE_CONTROL, "public, max-age=86400".to_string()),
	];
	
	Ok((headers, body))
}

pub async fn get_style(
	Path(name): Path<String>,
) -> AppResult<impl IntoResponse> {
	let sanitized_name = sanitize_filename::sanitize(&name);
	if sanitized_name.is_empty() || sanitized_name != name {
		return Err(AppError::BadRequest("invalid static path".into()));
	}
	
	let mut buf = PathBuf::from("static/css/dist");
	buf.push(&name);
	
	let body = read_file_to_string(&buf)
		.await
		.map_err(|_| AppError::NotFound)?;
	
	let headers = [
		(header::CONTENT_TYPE, "text/css".to_string()),
		(header::CACHE_CONTROL, "public, max-age=86400".to_string()),
	];
	
	Ok((headers, body))
}

pub async fn get_script(
	Path(name): Path<String>,
) -> AppResult<impl IntoResponse> {
	let sanitized_name = sanitize_filename::sanitize(&name);
	if sanitized_name.is_empty() || sanitized_name != name {
		return Err(AppError::BadRequest("invalid static path".into()));
	}
	
	let mut buf = PathBuf::from("static/js");
	buf.push(&name);
	
	let body = read_file_to_string(&buf)
		.await
		.map_err(|_| AppError::NotFound)?;
	
	let headers = [
		(header::CONTENT_TYPE, "text/javascript".to_string()),
		(header::CACHE_CONTROL, "public, max-age=86400".to_string()),
	];
	
	Ok((headers, body))
}
