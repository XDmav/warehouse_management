use axum::extract::{Path, State};
use axum::response::{IntoResponse};
use axum::Json;
use serde::Serialize;
use sqlx::Row;
use std::sync::Arc;
use axum_extra::extract::CookieJar;
use crate::app_error::{AppError, AppResult};
use crate::useful_funcs::{get_user, SharedStateStruct};

#[derive(Serialize)]
pub struct StockResponse {
    stock: i64,
}

pub async fn goods_stock(
    jar: CookieJar,
    Path(goods_id): Path<i64>,
    State(state): State<Arc<SharedStateStruct>>,
) -> AppResult<impl IntoResponse> {
    let user_id = get_user(&jar, &state).await;
    
    if user_id.is_none() {
        return Err(AppError::Unauthorized);
    }
    
    let row = sqlx::query(
        "SELECT COALESCE(SUM(quantity),0) as stock
        FROM warehouse_goods
        WHERE goods_id = $1",
    )
    .bind(goods_id)
    .fetch_one(&state.pool)
    .await?;

    let stock: i64 = row.try_get("stock").map_err(|e| AppError::Internal(format!("column 'stock': {e}")))?;

   Ok(Json(StockResponse { stock }))
}

#[derive(Serialize)]
pub struct GoodsInfo {
    id: i64,
    name: String,
    price: f64,
}

pub async fn goods_list(
    jar: CookieJar,
    State(state): State<Arc<SharedStateStruct>>
) -> AppResult<impl IntoResponse> {
    let user_id = get_user(&jar, &state).await;
    
    if user_id.is_none() {
        return Err(AppError::Unauthorized);
    }
    
    let rows = sqlx::query("SELECT goods_id,name,price::float8 FROM goods ORDER BY name")
        .fetch_all(&state.pool)
        .await?;

    let mut result = Vec::new();

    for r in rows {
        result.push(GoodsInfo {
            id:    r.try_get("goods_id").map_err(|e| AppError::Internal(e.to_string()))?,
            name:  r.try_get("name").map_err(|e| AppError::Internal(e.to_string()))?,
            price: r.try_get("price").map_err(|e| AppError::Internal(e.to_string()))?,
        });
    }
    
    Ok(Json(result))
}
