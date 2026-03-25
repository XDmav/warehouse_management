use std::ops::DerefMut;
use crate::useful_funcs::{check_permission, get_user, SharedStateStruct};
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::Form;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use sqlx::Row;
use std::sync::Arc;
use axum::http::StatusCode;
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
	match user_id {
		Some(user_id) => {
			if !check_permission(&state, user_id, "CREATE").await {
				return Err((StatusCode::UNAUTHORIZED, "Unauthorized").into_response());
			}
		}
		None => return Err((StatusCode::UNAUTHORIZED, "Unauthorized").into_response()),
	}
	
	if data.goods_id.is_empty()
		|| data.goods_id.len() != data.quantity.len()
		|| data.goods_id.len() != data.price.len()
		|| data.goods_id.len() != data.discount.len()
	{
		return Err((StatusCode::BAD_REQUEST, Html("Некорректные данные чека")).into_response());
	}
	
	let mut tx = match state.pool.begin().await {
		Ok(tx) => tx,
		Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Html("Ошибка при создании")).into_response()),
	};
	
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
		.fetch_one(tx.deref_mut())
		.await;
	
	let rec = match rec {
		Ok(rec) => rec,
		Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Html("Ошибка при создании")).into_response()),
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
			.execute(tx.deref_mut())
			.await;
		
		if result.is_err() {
			return Err((StatusCode::INTERNAL_SERVER_ERROR, Html("Ошибка при создании")).into_response());
		}
	}
	
	let result = tx.commit().await;
	if result.is_err() {
		return Err((StatusCode::INTERNAL_SERVER_ERROR, Html("Ошибка при создании")).into_response());
	}
	
	Ok(Html("Чек успешно создан".to_string()))
}
