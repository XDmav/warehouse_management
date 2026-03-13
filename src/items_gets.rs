pub mod simple_gets;

use std::path::PathBuf;
use std::sync::Arc;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse};
use axum_extra::extract::CookieJar;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::items_gets::simple_gets::{bad_request, fallback, SharedStateStruct, read_file_to_string};

pub async fn get_image(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Path(name): Path<String>
) -> impl IntoResponse {
	let sanitized_name = sanitize_filename::sanitize(&name);
	if sanitized_name.is_empty() || sanitized_name != name {
		return Err(bad_request(jar, State(state)).await);
	}
	
	let mut buf = PathBuf::from("static/images");
	buf.push(&sanitized_name);
	
	let filename = match buf.file_name() {
		Some(name) => name,
		None => return Err(bad_request(jar, State(state)).await)
	};
	let file = match File::open(&buf).await {
		Ok(file) => file,
		Err(_) => return Err(fallback(jar, State(state)).await)
	};
	let content_type = match mime_guess::from_path(&name).first_raw() {
		Some(mime) => mime,
		None => return Err(bad_request(jar, State(state)).await)
	};
	
	let stream = ReaderStream::new(file);
	let body = Body::from_stream(stream);
	
	let headers = [
		(header::CONTENT_TYPE, content_type.to_string()),
		(
			header::CONTENT_DISPOSITION,
			format!("attachment; filename=\"{:?}\"", filename),
		),
	];
	Ok((headers, body))
}

pub async fn get_style(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Path(name): Path<String>
) -> Result<impl IntoResponse, (StatusCode, Html<String>)> {
	let sanitized_name = sanitize_filename::sanitize(&name);
	if sanitized_name.is_empty() || sanitized_name != name {
		return Err(bad_request(jar, State(state)).await);
	}
	
	let mut buf = PathBuf::from("static/css/dist");
	buf.push(&name);
	
	let headers = [(header::CONTENT_TYPE, "text/css".to_string())];
	let body = match read_file_to_string(&buf).await {
		Ok(body) => body,
		Err(_) => {
			return Err(fallback(jar, State(state)).await)
		}
	};
	
	Ok((headers, body))
}

pub async fn get_script(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Path(name): Path<String>
) -> Result<impl IntoResponse, (StatusCode, Html<String>)> {
	let sanitized_name = sanitize_filename::sanitize(&name);
	if sanitized_name.is_empty() || sanitized_name != name {
		return Err(bad_request(jar, State(state)).await);
	}
	
	let mut buf = PathBuf::from("static/js");
	buf.push(&name);
	
	let headers = [(header::CONTENT_TYPE, "text/javascript".to_string())];
	let body = match read_file_to_string(&buf).await {
		Ok(body) => body,
		Err(_) => {
			return Err(fallback(jar, State(state)).await)
		}
	};
	
	Ok((headers, body))
}