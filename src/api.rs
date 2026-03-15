use axum::extract::{Path, State};
use axum::response::{IntoResponse};
use axum::Json;
use serde::Serialize;
use sqlx::Row;
use std::sync::Arc;
use axum_extra::extract::CookieJar;
use crate::useful_funcs::{get_user, SharedStateStruct};

#[derive(Serialize)]
pub struct StockResponse {
    stock: i64,
}

pub async fn goods_stock(
    jar: CookieJar,
    Path(goods_id): Path<i64>,
    State(state): State<Arc<SharedStateStruct>>,
) -> impl IntoResponse {
    let user_id = get_user(&jar, &state).await;
    
    if user_id.is_none() {
        return Err(());
    }
    
    let row = sqlx::query(
        "SELECT COALESCE(SUM(quantity),0) as stock
        FROM warehouse_goods
        WHERE goods_id = $1",
    )
    .bind(goods_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();

    let stock: i64 = row.get("stock");

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
) -> impl IntoResponse {
    let user_id = get_user(&jar, &state).await;
    
    if user_id.is_none() {
        return Err(());
    }
    
    let rows = sqlx::query("SELECT goods_id,name,price::float8 FROM goods ORDER BY name")
        .fetch_all(&state.pool)
        .await
        .unwrap();

    let mut result = Vec::new();

    for r in rows {
        result.push(GoodsInfo {
            id: r.get("goods_id"),
            name: r.get("name"),
            price: r.get("price"),
        });
    }
    
    Ok(Json(result))
}
