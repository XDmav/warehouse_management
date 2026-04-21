use axum::{
	extract::Request,
	http::{Method, StatusCode},
	middleware::Next,
	response::Response,
};
use axum_extra::extract::{CookieJar, cookie::{Cookie, SameSite}};
use base16ct::lower;
use rand::{rngs::StdRng, Rng};

pub async fn csrf_middleware(jar: CookieJar, req: Request, next: Next) -> Result<(CookieJar, Response), StatusCode> {
	if matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS) {
		let jar = if jar.get("CSRF-TOKEN").is_none() {
			let mut buf = [0; 32];
			let mut rng: StdRng = rand::make_rng();
			rng.fill_bytes(&mut buf);
			
			let token = lower::encode_string(&buf);
			
			let mut c = Cookie::new("CSRF-TOKEN", token);
			c.set_secure(true);
			c.set_http_only(false);
			c.set_same_site(SameSite::Lax);
			c.set_path("/");
			jar.add(c)
		} else { jar };
		
		let resp = next.run(req).await;
		
		return Ok((jar, resp));
	}
	
	let cookie_token = jar.get("CSRF-TOKEN").map(|c| c.value().to_string());
	let header_token = req.headers()
		.get("X-CSRF-Token")
		.and_then(|h| h.to_str().ok())
		.map(|s| s.to_string());
	
	match (cookie_token, header_token) {
		(Some(a), Some(b)) if a == b => Ok((jar, next.run(req).await)),
		_ => Err(StatusCode::FORBIDDEN),
	}
}