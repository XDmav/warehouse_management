use std::ops::DerefMut;
use crate::useful_funcs::{check_permission, get_user, SharedStateStruct};
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::Form;
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use sqlx::Row;
use std::sync::Arc;
use time::Date;
use crate::app_error::{AppError, AppResult};

#[derive(Deserialize)]
pub struct CreateReceipt {
	receipt_date: Date,
	payment_type: String,
	cashier_id: i64,
	delivery_type: String,
	store_id: Option<i64>,
	card_number: Option<String>,
	
	goods_id: Vec<i64>,
	quantity: Vec<i32>,
}

fn map_insert_receipt_error(e: sqlx::Error) -> AppError {
	if let sqlx::Error::Database(ref db_err) = e {
		if db_err.is_foreign_key_violation() {
			return AppError::BadRequest(
				"Один из справочных параметров не существует \
                (тип оплаты, кассир, тип доставки или магазин)".into()
			);
		}
		if db_err.is_check_violation() {
			return AppError::BadRequest("Нарушены ограничения на поля чека".into());
		}
	}
	AppError::Db(e)
}

pub async fn create_receipt(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Form(data): Form<CreateReceipt>,
) -> AppResult<impl IntoResponse> {
	let user_id = get_user(&jar, &state).await.ok_or(AppError::Unauthorized)?;
	if !check_permission(&state, user_id, "CREATE").await {
		return Err(AppError::Unauthorized);
	}
	
	if data.goods_id.is_empty() || data.goods_id.len() != data.quantity.len() {
		return Err(AppError::BadRequest("Некорректные данные чека".into()));
	}
	if data.quantity.iter().any(|&q| q <= 0) {
		return Err(AppError::BadRequest("Количество должно быть > 0".into()));
	}
	
	if data.goods_id.len() > 1000 {
		return Err(AppError::BadRequest("Слишком много позиций в чеке".into()));
	}
	
	let mut seen = std::collections::HashSet::new();
	for &gid in &data.goods_id {
		if !seen.insert(gid) {
			return Err(AppError::BadRequest("Один товар не может встречаться в чеке дважды".into()));
		}
	}
	
	let mut tx = state.pool.begin().await?;
	
	let warehouse_code: Option<String> = if let Some(store_id) = data.store_id {
		sqlx::query("SELECT warehouse_code FROM stores WHERE store_id = $1")
			.bind(store_id)
			.fetch_optional(tx.deref_mut())
			.await?
			.and_then(|row| row.try_get::<Option<String>, _>("warehouse_code").ok())
			.flatten()
	} else {
		None
	};
	
	let receipt_id: i64 = sqlx::query(
		"INSERT INTO receipts (receipt_date, payment_type, cashier_id, delivery_type, store_id)
         VALUES ($1, $2, $3, $4, $5) RETURNING receipt_id"
	)
		.bind(data.receipt_date)
		.bind(&data.payment_type)
		.bind(data.cashier_id)
		.bind(&data.delivery_type)
		.bind(data.store_id)
		.fetch_one(tx.deref_mut())
		.await
		.map_err(map_insert_receipt_error)?
		.try_get("receipt_id")
		.map_err(|e| AppError::Internal(format!("column 'receipt_id': {e}")))?;
	
	for i in 0..data.goods_id.len() {
		let goods_id = data.goods_id[i];
		let qty = data.quantity[i];
		
		let price: f64 = sqlx::query("SELECT price::float8 AS price FROM goods WHERE goods_id = $1")
			.bind(goods_id)
			.fetch_optional(tx.deref_mut())
			.await?
			.ok_or_else(|| AppError::BadRequest(format!("Товар {goods_id} не найден")))?
			.try_get("price")
			.map_err(|e| AppError::Internal(format!("column 'price': {e}")))?;
		
		let discount: f64 = match &data.card_number {
			Some(card) if !card.trim().is_empty() => {
				sqlx::query(
					"SELECT discount::float8 AS discount
                     FROM card_goods_discounts WHERE card_number = $1 AND goods_id = $2"
				)
					.bind(card)
					.bind(goods_id)
					.fetch_optional(tx.deref_mut())
					.await?
					.and_then(|row| row.try_get("discount").ok())
					.unwrap_or(0.0)
			}
			_ => 0.0,
		};
		
		sqlx::query(
			"INSERT INTO receipt_items (receipt_id, goods_id, quantity, price, discount)
             VALUES ($1, $2, $3, $4, $5)"
		)
			.bind(receipt_id)
			.bind(goods_id)
			.bind(qty)
			.bind(price)
			.bind(discount)
			.execute(tx.deref_mut())
			.await
			.map_err(|e| AppError::Internal(format!("insert receipt_item: {e}")))?;
		
		if let Some(wh) = &warehouse_code {
			let upd = sqlx::query("
					UPDATE warehouse_goods SET quantity = quantity - $1
                    WHERE warehouse_code = $2 AND goods_id = $3")
				.bind(qty).bind(wh).bind(goods_id)
				.execute(tx.deref_mut())
				.await;
			
			match upd {
				Ok(result) if result.rows_affected() == 0 => {
					return Err(AppError::BadRequest(format!(
						"Товара {goods_id} нет на складе {wh}"
					)));
				}
				Ok(_) => {}
				Err(sqlx::Error::Database(db_err)) if db_err.is_check_violation() => {
					return Err(AppError::BadRequest(format!(
						"Недостаточно товара {goods_id} на складе {wh}"
					)));
				}
				Err(e) => return Err(AppError::Db(e)),
			}
		}
	}
	
	tx.commit().await?;
	
	Ok(Html("Чек успешно создан".to_string()))
}
