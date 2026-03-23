use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::IntoResponse;
use axum_extra::extract::CookieJar;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::pages_gets::errors::{bad_request, not_found};
use crate::useful_funcs::{read_file_to_string, SharedStateStruct};

pub async fn get_image(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Path(name): Path<String>,
) -> impl IntoResponse {
	let sanitized_name = sanitize_filename::sanitize(&name);
	if sanitized_name.is_empty() || sanitized_name != name {
		return Err(bad_request(jar, State(state)).await.into_response());
	}
	
	let mut buf = PathBuf::from("static/images");
	buf.push(&sanitized_name);
	
	let file = match File::open(&buf).await {
		Ok(file) => file,
		Err(_) => return Err(not_found(jar, State(state)).await.into_response()),
	};
	
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
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Path(name): Path<String>,
) -> impl IntoResponse {
	let sanitized_name = sanitize_filename::sanitize(&name);
	if sanitized_name.is_empty() || sanitized_name != name {
		return Err(bad_request(jar, State(state)).await.into_response());
	}
	
	let mut buf = PathBuf::from("static/css/dist");
	buf.push(&name);
	
	let headers = [(header::CONTENT_TYPE, "text/css".to_string())];
	let body = match read_file_to_string(&buf).await {
		Ok(body) => body,
		Err(_) => return Err(not_found(jar, State(state)).await.into_response()),
	};
	
	Ok((headers, body))
}

pub async fn get_script(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Path(name): Path<String>,
) -> impl IntoResponse {
	let sanitized_name = sanitize_filename::sanitize(&name);
	if sanitized_name.is_empty() || sanitized_name != name {
		return Err(bad_request(jar, State(state)).await.into_response());
	}
	
	let mut buf = PathBuf::from("static/js");
	buf.push(&name);
	
	let headers = [(header::CONTENT_TYPE, "text/javascript".to_string())];
	let body = match read_file_to_string(&buf).await {
		Ok(body) => body,
		Err(_) => return Err(not_found(jar, State(state)).await.into_response()),
	};
	
	Ok((headers, body))
}
