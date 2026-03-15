use crate::useful_funcs::{get_user, SharedStateStruct};
use axum::extract::State;
use axum::response::{Html, IntoResponse, Redirect};
use axum::Form;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use sqlx::Row;
use std::sync::Arc;
use time::Date;

#[derive(Deserialize)]
pub struct CreateReceipt {
	receipt_date: Date,
	payment_type: String,
	cashier_id: i64,
	delivery_type: String,
	store_id: Option<i64>,
	
	goods_id: Vec<i64>,
	quantity: Vec<i32>,
	price: Vec<f64>,
	discount: Vec<f64>,
}

pub async fn create_receipt(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Form(data): Form<CreateReceipt>,
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	
	if user_id.is_none() {
		return Err(Redirect::to("/login"));
	}
	
	let rec = sqlx::query(
		"INSERT INTO receipts
		(receipt_date,payment_type,cashier_id,delivery_type,store_id)
		VALUES ($1,$2,$3,$4,$5)
		RETURNING receipt_id",
	)
		.bind(data.receipt_date)
		.bind(&data.payment_type)
		.bind(data.cashier_id)
		.bind(&data.delivery_type)
		.bind(data.store_id)
		.fetch_one(&state.pool)
		.await;
	
	let rec = match rec {
		Ok(rec) => rec,
		Err(e) => return Ok(Html(e.to_string())),
	};
	
	let receipt_id: i64 = rec.get("receipt_id");
	
	for i in 0..data.goods_id.len() {
		let result = sqlx::query(
			"INSERT INTO receipt_items
			(receipt_id,goods_id,quantity,price,discount)
			VALUES ($1,$2,$3,$4,$5)",
		)
			.bind(receipt_id)
			.bind(data.goods_id[i])
			.bind(data.quantity[i])
			.bind(data.price[i])
			.bind(data.discount[i])
			.execute(&state.pool)
			.await;
		
		if result.is_err() {
			return Ok(Html("Ошибка при создании".to_string()));
		}
	}
	
	Ok(Html("Чек успешно создан".to_string()))
}
