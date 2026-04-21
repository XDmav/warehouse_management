use axum::extract::{Query, State};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use sqlx::Row;
use std::path::PathBuf;
use std::sync::Arc;
use serde::Deserialize;
use html_escape::encode_safe;
use crate::app_error::{AppError, AppResult};
use crate::pages_gets::errors::unauthorized;
use crate::useful_funcs::{check_permission, get_user, read_file_to_string, replace_html_in_html, replace_text_in_html, SharedStateStruct};

pub mod errors;
pub mod static_gets;

#[derive(Deserialize)]
pub struct AuthQuery {
	error: Option<String>,
	success: Option<String>,
}

fn login_error_message(code: Option<&str>) -> &'static str {
	match code {
		Some("invalid")    => "Неверный email или пароль",
		Some("rate_limit") => "Слишком много попыток, попробуйте через 15 минут",
		_ => "",
	}
}

fn registration_error_message(code: Option<&str>) -> &'static str {
	match code {
		Some("invalid_email") => "Некорректный email",
		Some("email_taken")   => "Этот email уже зарегистрирован",
		Some("too_short")     => "Пароль должен быть не короче 12 символов",
		Some("too_long")      => "Пароль слишком длинный (максимум 128 символов)",
		Some("no_digit")      => "Пароль должен содержать хотя бы одну цифру",
		Some("no_upper")      => "Пароль должен содержать хотя бы одну заглавную букву",
		Some("no_lower")      => "Пароль должен содержать хотя бы одну строчную букву",
		Some("no_special")    => "Пароль должен содержать хотя бы один спецсимвол",
		_ => "",
	}
}

fn registration_success_message(code: Option<&str>) -> &'static str {
	match code {
		Some(_) => "Пользователь успешно зарегистрирован",
		None    => "",
	}
}

pub async fn login(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Query(q): Query<AuthQuery>,
) -> AppResult<Response> {
	if get_user(&jar, &state).await.is_some() {
		return Ok(Redirect::to("/").into_response());
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/login.html")).await?;
	
	let error_text = login_error_message(q.error.as_deref());
	page = replace_html_in_html(page, "error_message", error_text);
	
	Ok(Html(page).into_response())
}

pub async fn registration(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
	Query(q): Query<AuthQuery>,
) -> AppResult<Response> {
	let user_id = match get_user(&jar, &state).await {
		Some(id) => id,
		None => return Ok(Redirect::to("/login").into_response()),
	};
	
	if !check_permission(&state, user_id, "REG").await {
		return Ok(unauthorized(jar, State(state)).await.into_response());
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/registration.html")).await?;
	
	let error_text = registration_error_message(q.error.as_deref());
	let success_text = registration_success_message(q.success.as_deref());
	
	page = replace_html_in_html(page, "error_message", error_text);
	page = replace_html_in_html(page, "success_message", success_text);
	
	Ok(Html(page).into_response())
}

pub async fn home(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> AppResult<impl IntoResponse> {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Ok(Redirect::to("/login").into_response());
	}
	let body = read_file_to_string(&PathBuf::from("templates/index.html")).await?;
	Ok(Html(body).into_response())
}

pub async fn stats(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> AppResult<impl IntoResponse> {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Ok(Redirect::to("/login").into_response());
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/stats.html")).await?;
	
	let receipts: i64 = sqlx::query("SELECT COUNT(*) as count FROM receipts")
		.fetch_one(&state.pool)
		.await?
		.try_get("count")
		.map_err(|e| AppError::Internal(e.to_string()))?;
	
	let revenue: Option<f64> = sqlx::query("SELECT SUM(quantity*price*(1-discount/100.0)) as sum FROM receipt_items")
		.fetch_one(&state.pool)
		.await?
		.try_get("sum")
		.ok();
	
	let goods: i64 = sqlx::query("SELECT COUNT(*) as count FROM goods")
		.fetch_one(&state.pool)
		.await?
		.try_get("count")
		.map_err(|e| AppError::Internal(e.to_string()))?;
	
	page = replace_text_in_html(page, "receipts", &receipts.to_string());
	page = replace_text_in_html(page, "revenue", &format!("{:.2}", revenue.unwrap_or(0.0)));
	page = replace_text_in_html(page, "goods", &goods.to_string());
	
	Ok(Html(page).into_response())
}

pub async fn stats_sales(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> AppResult<impl IntoResponse> {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Ok(Redirect::to("/login").into_response());
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/stats_sales.html")).await?;
	
	let rows = sqlx::query(
		"SELECT g.name, SUM(ri.quantity) as sold
         FROM receipt_items ri
         JOIN goods g ON g.goods_id = ri.goods_id
         GROUP BY g.goods_id
         ORDER BY sold DESC
         LIMIT 10",
	)
		.fetch_all(&state.pool)
		.await?;
	
	let mut list = String::new();
	
	for r in rows {
		let name: String = r.try_get("name").map_err(|e| AppError::Internal(e.to_string()))?;
		let sold: i64 = r.try_get("sold").map_err(|e| AppError::Internal(e.to_string()))?;
		
		list.push_str(&format!("<li>{} — {} шт.</li>", encode_safe(name.as_str()), sold));
	}
	
	let rows = sqlx::query(
		"SELECT
		    to_char(date_trunc('month', r.receipt_date),'YYYY-MM') AS month,
		    SUM(ri.quantity * ri.price * (1 - ri.discount/100.0))::float8 AS revenue
		FROM receipts r
		JOIN receipt_items ri ON ri.receipt_id = r.receipt_id
		GROUP BY month
		ORDER BY month DESC
		LIMIT 12",
	)
		.fetch_all(&state.pool)
		.await?;
	
	let mut monthly_stats = String::new();
	
	for r in rows {
		let month: String = r.try_get("month").map_err(|e| AppError::Internal(e.to_string()))?;
		let revenue: f64 = r.try_get("revenue").map_err(|e| AppError::Internal(e.to_string()))?;
		
		monthly_stats.push_str(&format!("<li>{} — {:.2}</li>", encode_safe(month.as_str()), revenue));
	}
	
	let avg_check: f64 = sqlx::query(
		"SELECT COALESCE(AVG(total)::float8,0) as avg
         FROM (
             SELECT SUM(quantity*price*(1-discount/100.0)) as total
             FROM receipt_items
             GROUP BY receipt_id
         ) t",
	)
		.fetch_one(&state.pool)
		.await?
		.try_get("avg")
		.map_err(|e| AppError::Internal(e.to_string()))?;
	
	let items_sold: i64 = sqlx::query("SELECT COALESCE(SUM(quantity),0) as sum FROM receipt_items")
		.fetch_one(&state.pool)
		.await?
		.try_get("sum")
		.map_err(|e| AppError::Internal(e.to_string()))?;
	
	let orders_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM orders")
		.fetch_one(&state.pool)
		.await?
		.try_get("count")
		.map_err(|e| AppError::Internal(e.to_string()))?;
	
	page = replace_html_in_html(page, "top_goods", &list);
	page = replace_html_in_html(page, "monthly_revenue", &monthly_stats);
	page = replace_text_in_html(page, "avg_check", &format!("{:.2}", avg_check));
	page = replace_text_in_html(page, "items_sold", &items_sold.to_string());
	page = replace_text_in_html(page, "orders_count", &orders_count.to_string());
	
	Ok(Html(page).into_response())
}

pub async fn stats_goods(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> AppResult<impl IntoResponse> {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Ok(Redirect::to("/login").into_response());
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/stats_goods.html")).await?;
	
	let rows = sqlx::query(
	"SELECT
            g.name,
            SUM(ri.quantity * ri.price * (1 - ri.discount / 100.0))::float8 AS revenue
        FROM receipt_items ri
        JOIN goods g ON g.goods_id = ri.goods_id
        GROUP BY g.goods_id, g.name
        ORDER BY revenue DESC, g.name
        LIMIT 10"
	)
		.fetch_all(&state.pool)
		.await?;
	
	let mut list = String::new();
	
	for r in rows {
		let name: String = r.try_get("name").map_err(|e| AppError::Internal(e.to_string()))?;
		let revenue: f64 = r.try_get("revenue").map_err(|e| AppError::Internal(e.to_string()))?;
		
		list.push_str(&format!("<li>{} — {:.2}</li>", encode_safe(name.as_str()), revenue));
	}
	
	if list.is_empty() {
		list.push_str("<li>Нет данных о продажах</li>");
	}
	
	let rows = sqlx::query(
	"WITH revenue_per_goods AS (
            SELECT
                g.goods_id,
                g.name,
                SUM(ri.quantity * ri.price * (1 - ri.discount / 100.0))::float8 AS revenue
            FROM receipt_items ri
            JOIN goods g ON g.goods_id = ri.goods_id
            GROUP BY g.goods_id, g.name
        ),
        ranked AS (
            SELECT
                goods_id,
                name,
                revenue,
                revenue / NULLIF(SUM(revenue) OVER (), 0) AS share,
                SUM(revenue) OVER (
                    ORDER BY revenue DESC, name
                    ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
                ) / NULLIF(SUM(revenue) OVER (), 0) AS cumulative_share
            FROM revenue_per_goods
        )
        SELECT
            name,
            revenue,
            COALESCE(share, 0)::float8 AS share,
            COALESCE(cumulative_share, 0)::float8 AS cumulative_share,
            CASE
                WHEN COALESCE(cumulative_share, 0) <= 0.80 THEN 'A'
                WHEN COALESCE(cumulative_share, 0) <= 0.95 THEN 'B'
                ELSE 'C'
            END AS category
        FROM ranked
        ORDER BY revenue DESC, name
        LIMIT 20"
	)
		.fetch_all(&state.pool)
		.await?;
	
	let mut abc_list = String::new();
	
	for r in rows {
		let name: String = r.try_get("name").map_err(|e| AppError::Internal(e.to_string()))?;
		let revenue: f64 = r.try_get("revenue").map_err(|e| AppError::Internal(e.to_string()))?;
		let share: f64 = r.try_get("share").map_err(|e| AppError::Internal(e.to_string()))?;
		let cumulative_share: f64 = r.try_get("cumulative_share").map_err(|e| AppError::Internal(e.to_string()))?;
		let category: String = r.try_get("category").map_err(|e| AppError::Internal(e.to_string()))?;
		
		abc_list.push_str(&format!(
			"<li>[{}] {} — {:.2} (доля: {:.2}%, накопительно: {:.2}%)</li>",
			encode_safe(category.as_str()),
			encode_safe(name.as_str()),
			revenue,
			share * 100.0,
			cumulative_share * 100.0
		));
	}
	
	if abc_list.is_empty() {
		abc_list.push_str("<li>Недостаточно данных для ABC-анализа</li>");
	}
	
	let rows = sqlx::query(
	"WITH monthly_sales AS (
            SELECT
                g.goods_id,
                g.name,
                date_trunc('month', r.receipt_date)::date AS month,
                SUM(ri.quantity) AS qty
            FROM receipt_items ri
            JOIN receipts r ON r.receipt_id = ri.receipt_id
            JOIN goods g ON g.goods_id = ri.goods_id
            GROUP BY g.goods_id, g.name, date_trunc('month', r.receipt_date)::date
        ),
        ranked_months AS (
            SELECT
                goods_id,
                name,
                month,
                qty,
                DENSE_RANK() OVER (ORDER BY month DESC) AS month_rank
            FROM monthly_sales
        ),
        latest AS (
            SELECT goods_id, name, qty
            FROM ranked_months
            WHERE month_rank = 1
        ),
        previous AS (
            SELECT goods_id, name, qty
            FROM ranked_months
            WHERE month_rank = 2
        )
        SELECT
            l.name,
            l.qty AS current_month,
            p.qty AS prev_month
        FROM latest l
        JOIN previous p ON p.goods_id = l.goods_id
        WHERE l.qty < p.qty
        ORDER BY (p.qty - l.qty) DESC, l.name
        LIMIT 10"
	)
		.fetch_all(&state.pool)
		.await?;
	
	let mut falling_goods = String::new();
	
	for r in rows {
		let name: String = r.try_get("name").map_err(|e| AppError::Internal(e.to_string()))?;
		let current: i64 = r.try_get("current_month").map_err(|e| AppError::Internal(e.to_string()))?;
		let prev: i64 = r.try_get("prev_month").map_err(|e| AppError::Internal(e.to_string()))?;
		
		falling_goods.push_str(&format!(
			"<li>{} — {} → {}</li>",
			encode_safe(name.as_str()), prev, current
		));
	}
	
	if falling_goods.is_empty() {
		falling_goods.push_str("<li>Нет данных о снижении продаж</li>");
	}
	
	let goods_total: i64 = sqlx::query("SELECT COUNT(*) AS count FROM goods")
		.fetch_one(&state.pool)
		.await?
		.try_get("count")
		.map_err(|e| AppError::Internal(e.to_string()))?;
	
	let goods_with_sales: i64 = sqlx::query("SELECT COUNT(DISTINCT goods_id) AS count FROM receipt_items")
		.fetch_one(&state.pool)
		.await?
		.try_get("count")
		.map_err(|e| AppError::Internal(e.to_string()))?;
	
	let avg_price: f64 = sqlx::query("SELECT COALESCE(AVG(price)::float8, 0) AS avg FROM goods")
		.fetch_one(&state.pool)
		.await?
		.try_get("avg")
		.map_err(|e| AppError::Internal(e.to_string()))?;
	
	page = replace_html_in_html(page, "goods_revenue", &list);
	page = replace_html_in_html(page, "abc_goods", &abc_list);
	page = replace_html_in_html(page, "falling_goods", &falling_goods);
	page = replace_text_in_html(page, "goods_total", &goods_total.to_string());
	page = replace_text_in_html(page, "goods_with_sales", &goods_with_sales.to_string());
	page = replace_text_in_html(page, "avg_price", &format!("{:.2}", avg_price));
	
	Ok(Html(page).into_response())
}

pub async fn stats_warehouse(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> AppResult<impl IntoResponse> {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Ok(Redirect::to("/login").into_response());
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/stats_warehouse.html")).await?;
	
	let receipts: i64 = sqlx::query("SELECT COUNT(*) as count FROM goods_receipts")
		.fetch_one(&state.pool)
		.await?
		.try_get("count")
		.map_err(|e| AppError::Internal(e.to_string()))?;
	
	let writeoffs: i64 = sqlx::query("SELECT COUNT(*) as count FROM writeoff_acts")
		.fetch_one(&state.pool)
		.await?
		.try_get("count")
		.map_err(|e| AppError::Internal(e.to_string()))?;
	
	let stock_goods: i64 = sqlx::query("SELECT COALESCE(SUM(quantity),0) as count FROM warehouse_goods")
		.fetch_one(&state.pool)
		.await?
		.try_get("count")
		.map_err(|e| AppError::Internal(e.to_string()))?;
	
	let suppliers: i64 = sqlx::query("SELECT COUNT(*) as count FROM suppliers")
		.fetch_one(&state.pool)
		.await?
		.try_get("count")
		.map_err(|e| AppError::Internal(e.to_string()))?;
	
	let warehouses: i64 = sqlx::query("SELECT COUNT(*) as count FROM warehouses")
		.fetch_one(&state.pool)
		.await?
		.try_get("count")
		.map_err(|e| AppError::Internal(e.to_string()))?;
	
	page = replace_text_in_html(page, "receipts", &receipts.to_string());
	page = replace_text_in_html(page, "writeoffs", &writeoffs.to_string());
	page = replace_text_in_html(page, "stock_goods", &stock_goods.to_string());
	page = replace_text_in_html(page, "suppliers", &suppliers.to_string());
	page = replace_text_in_html(page, "warehouses", &warehouses.to_string());
	
	Ok(Html(page).into_response())
}

pub async fn create_receipt_page(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>,
) -> AppResult<impl IntoResponse> {
	let user_id = get_user(&jar, &state).await;
	match user_id {
		Some(user_id) => {
			if !check_permission(&state, user_id, "CREATE").await {
				return Ok(Redirect::to("/login").into_response());
			}
		}
		None => return Ok(Redirect::to("/login").into_response())
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/receipt_create.html")).await?;
	
	let payment_rows = sqlx::query("SELECT payment_type FROM payment_types")
		.fetch_all(&state.pool)
		.await?;
	
	let mut payment_html = String::new();
	
	for r in payment_rows {
		let payment_type: String = r.try_get("payment_type").map_err(|e| AppError::Internal(e.to_string()))?;
		let payment_type = encode_safe(payment_type.as_str());
		
		payment_html.push_str(&format!("<option value=\"{}\">{}</option>", payment_type, payment_type));
	}
	
	let cashier_rows = sqlx::query("SELECT cashier_id,surname,first_name FROM cashiers")
		.fetch_all(&state.pool)
		.await?;
	
	let mut cashier_html = String::new();
	
	for r in cashier_rows {
		let id: i64 = r.try_get("cashier_id").map_err(|e| AppError::Internal(e.to_string()))?;
		let name: String = r.try_get("surname").map_err(|e| AppError::Internal(e.to_string()))?;
		let fname: String = r.try_get("first_name").map_err(|e| AppError::Internal(e.to_string()))?;
		
		cashier_html.push_str(&format!(
			"<option value=\"{}\">{} {}</option>",
			id, encode_safe(name.as_str()), encode_safe(fname.as_str())
		));
	}
	
	let delivery_rows = sqlx::query("SELECT delivery_type FROM delivery_types")
		.fetch_all(&state.pool)
		.await?;
	
	let mut delivery_html = String::new();
	
	for r in delivery_rows {
		let delivery_type: String = r.try_get("delivery_type").map_err(|e| AppError::Internal(e.to_string()))?;
		let delivery_type = encode_safe(delivery_type.as_str());
		
		delivery_html.push_str(&format!("<option value=\"{}\">{}</option>", delivery_type, delivery_type));
	}
	
	let store_rows = sqlx::query("SELECT store_id,address FROM stores")
		.fetch_all(&state.pool)
		.await?;
	
	let mut store_html = String::new();
	
	for r in store_rows {
		let id: i64 = r.try_get("store_id").map_err(|e| AppError::Internal(e.to_string()))?;
		let addr: String = r.try_get("address").map_err(|e| AppError::Internal(e.to_string()))?;
		
		store_html.push_str(&format!("<option value=\"{}\">{}</option>", id, encode_safe(addr.as_str())));
	}
	
	page = replace_html_in_html(page, "payment_types", &payment_html);
	page = replace_html_in_html(page, "cashiers", &cashier_html);
	page = replace_html_in_html(page, "delivery_types", &delivery_html);
	page = replace_html_in_html(page, "stores", &store_html);
	
	Ok(Html(page).into_response())
}
