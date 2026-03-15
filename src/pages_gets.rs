use std::path::PathBuf;
use std::sync::Arc;
use axum::extract::State;
use axum::response::{Html, IntoResponse, Redirect};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use sqlx::Row;
use time::OffsetDateTime;

use crate::pages_gets::errors::fallback;
use crate::useful_funcs::{check_permission, get_user, read_file_to_string, replace_in_html, SharedStateStruct};

pub mod static_gets;
pub mod errors;

pub async fn login(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_some() {
		return Err(Redirect::to("/"))
	}
	Ok(Html(read_file_to_string(&PathBuf::from("templates/login.html")).await.unwrap()))
}

pub async fn registration(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	match user_id {
		Some(user_id) => {
			if check_permission(&state, user_id, "REG").await {
				return Ok(Html(read_file_to_string(&PathBuf::from("templates/registration.html")).await.unwrap()))
			}
			Err(fallback(jar, State(state)).await.into_response())
		},
		None => Err(Redirect::to("/login").into_response())
	}
}

pub async fn logout(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	if let Some(val) = jar.get("SECURITY-COOKIE") {
        let val = val.value();
        let _ = sqlx::query("DELETE FROM web_page.cookies WHERE cookie = $1")
            .bind(val)
            .execute(&state.pool)
            .await;
    };
	
	let mut cookie = Cookie::new("SECURITY-COOKIE", "");
	cookie.set_secure(true);
	cookie.set_expires(OffsetDateTime::UNIX_EPOCH);
	(jar.add(cookie), Redirect::to("/"))
}

pub async fn home(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Err(Redirect::to("/login"))
	}
	Ok(Html(read_file_to_string(&PathBuf::from("templates/index.html")).await.unwrap()))
}

pub async fn stats(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Err(Redirect::to("/login"))
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/stats.html"))
		.await
		.unwrap();
	
	let receipts: i64 = sqlx::query("SELECT COUNT(*) as count FROM receipts")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	let revenue: Option<f64> = sqlx::query("SELECT SUM(quantity*price*(1-discount/100.0)) as sum FROM receipt_items")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.try_get("sum")
		.ok();
	
	let goods: i64 = sqlx::query("SELECT COUNT(*) as count FROM goods")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	page = replace_in_html(page,"receipts",&receipts.to_string()).await;
	page = replace_in_html(page,"revenue",&format!("{:.2}", revenue.unwrap_or(0.0))).await;
	page = replace_in_html(page,"goods", &goods.to_string()).await;
	
	Ok(Html(page))
}

pub async fn stats_sales(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Err(Redirect::to("/login"))
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/stats_sales.html")).await.unwrap();
	
	let rows = sqlx::query(
		"SELECT g.name, SUM(ri.quantity) as sold
         FROM receipt_items ri
         JOIN goods g ON g.goods_id = ri.goods_id
         GROUP BY g.goods_id
         ORDER BY sold DESC
         LIMIT 10"
	)
		.fetch_all(&state.pool)
		.await
		.unwrap();
	
	let mut list = String::new();
	
	for r in rows {
		let name: String = r.get("name");
		let sold: i64 = r.get("sold");
		
		list.push_str(&format!(
			"<li>{} — {} шт.</li>",
			name, sold
		));
	}
	
	let rows = sqlx::query(
	"SELECT
		    to_char(date_trunc('month', r.receipt_date),'YYYY-MM') AS month,
		    SUM(ri.quantity * ri.price * (1 - ri.discount/100.0))::float8 AS revenue
		FROM receipts r
		JOIN receipt_items ri ON ri.receipt_id = r.receipt_id
		GROUP BY month
		ORDER BY month DESC
		LIMIT 12"
	)
		.fetch_all(&state.pool)
		.await
		.unwrap();
	
	let mut monthly_stats = String::new();
	
	for r in rows {
		let month: String = r.get("month");
		let revenue: f64 = r.get("revenue");
		
		monthly_stats.push_str(&format!(
			"<li>{} — {:.2}</li>",
			month,
			revenue
		));
	}
	
	let avg_check: f64 = sqlx::query(
	"SELECT COALESCE(AVG(total)::float8,0) as avg
         FROM (
             SELECT SUM(quantity*price*(1-discount/100.0)) as total
             FROM receipt_items
             GROUP BY receipt_id
         ) t"
	)
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("avg");
	
	let items_sold: i64 = sqlx::query("SELECT COALESCE(SUM(quantity),0) as sum FROM receipt_items")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("sum");
	
	let orders_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM orders")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	page = replace_in_html(page,"top_goods",&list).await;
	page = replace_in_html(page,"monthly_revenue", &monthly_stats).await;
	page = replace_in_html(page,"avg_check",&format!("{:.2}",avg_check)).await;
	page = replace_in_html(page,"items_sold",&items_sold.to_string()).await;
	page = replace_in_html(page,"orders_count",&orders_count.to_string()).await;
	
	Ok(Html(page))
}

pub async fn stats_goods(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Err(Redirect::to("/login"))
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/stats_goods.html")).await.unwrap();
	
	let rows = sqlx::query(
	"SELECT g.name,
            SUM(ri.quantity*ri.price)::float8 as revenue
         FROM receipt_items ri
         JOIN goods g ON g.goods_id = ri.goods_id
         GROUP BY g.goods_id
         ORDER BY revenue DESC
         LIMIT 10"
	)
		.fetch_all(&state.pool)
		.await
		.unwrap();
	
	let mut list = String::new();
	
	for r in rows {
		let name: String = r.get("name");
		let revenue: f64 = r.get("revenue");
		
		list.push_str(&format!(
			"<li>{} — {:.2}</li>",
			name, revenue
		));
	}
	
	let rows = sqlx::query(
	"SELECT g.name,
		    SUM(ri.quantity*ri.price)::float8 AS revenue
		FROM receipt_items ri
		JOIN goods g ON g.goods_id = ri.goods_id
		GROUP BY g.goods_id
		ORDER BY revenue DESC
		LIMIT 20"
	)
		.fetch_all(&state.pool)
		.await
		.unwrap();
	
	let mut abc_list = String::new();
	
	for (index, r) in rows.into_iter().enumerate() {
		let name: String = r.get("name");
		let revenue: f64 = r.get("revenue");
		
		let category =
			if index < 5 {"A"}
			else if index < 12 {"B"}
			else {"C"};
		
		abc_list.push_str(&format!(
			"<li>[{}] {} — {:.2}</li>",
			category,
			name,
			revenue
		));
	}
	
	let rows = sqlx::query(
	"WITH sales AS (
		    SELECT
		        g.goods_id,
		        g.name,
		        date_trunc('month', r.receipt_date) AS month,
		        SUM(ri.quantity) AS qty
		    FROM receipt_items ri
		    JOIN receipts r ON r.receipt_id = ri.receipt_id
		    JOIN goods g ON g.goods_id = ri.goods_id
		    GROUP BY g.goods_id, month
		)
		
		SELECT
		    s1.name,
		    s1.qty AS current_month,
		    s2.qty AS prev_month
		FROM sales s1
		JOIN sales s2
		ON s1.goods_id = s2.goods_id
		AND s1.month = date_trunc('month', now())
		AND s2.month = date_trunc('month', now()) - interval '1 month'
		WHERE s1.qty < s2.qty
		ORDER BY (s2.qty - s1.qty) DESC
		LIMIT 10"
	)
		.fetch_all(&state.pool)
		.await
		.unwrap();
	
	let mut falling_goods = String::new();
	
	for r in rows {
		let name: String = r.get("name");
		let now: i64 = r.get("current_month");
		let prev: i64 = r.get("prev_month");
		
		falling_goods.push_str(&format!(
			"<li>{} — {} → {}</li>",
			name,
			prev,
			now
		));
	}
	
	let goods_total: i64 = sqlx::query("SELECT COUNT(*) as count FROM goods")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	let goods_with_sales: i64 = sqlx::query("SELECT COUNT(DISTINCT goods_id) as count FROM receipt_items")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	let avg_price: f64 = sqlx::query("SELECT COALESCE(AVG(price)::float8,0) as avg FROM goods")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("avg");
	
	page = replace_in_html(page,"goods_revenue",&list).await;
	page = replace_in_html(page,"abc_goods", &abc_list).await;
	page = replace_in_html(page,"falling_goods", &falling_goods).await;
	page = replace_in_html(page,"goods_total",&goods_total.to_string()).await;
	page = replace_in_html(page,"goods_with_sales",&goods_with_sales.to_string()).await;
	page = replace_in_html(page,"avg_price",&format!("{:.2}",avg_price)).await;
	
	Ok(Html(page))
}

pub async fn stats_warehouse(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar, &state).await;
	if user_id.is_none() {
		return Err(Redirect::to("/login"))
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/stats_warehouse.html")).await.unwrap();
	
	let receipts: i64 = sqlx::query("SELECT COUNT(*) as count FROM goods_receipts")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	let writeoffs: i64 = sqlx::query("SELECT COUNT(*) as count FROM writeoff_acts")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	let stock_goods: i64 = sqlx::query("SELECT COUNT(*) as count FROM goods")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	let suppliers: i64 = sqlx::query("SELECT COUNT(*) as count FROM suppliers")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	let warehouses: i64 = sqlx::query("SELECT COUNT(*) as count FROM warehouses")
		.fetch_one(&state.pool)
		.await
		.unwrap()
		.get("count");
	
	page = replace_in_html(page,"receipts",&receipts.to_string()).await;
	page = replace_in_html(page,"writeoffs",&writeoffs.to_string()).await;
	page = replace_in_html(page,"stock_goods",&stock_goods.to_string()).await;
	page = replace_in_html(page,"suppliers",&suppliers.to_string()).await;
	page = replace_in_html(page,"warehouses",&warehouses.to_string()).await;
	
	Ok(Html(page))
}

pub async fn create_receipt_page(
	jar: CookieJar,
	State(state): State<Arc<SharedStateStruct>>
) -> impl IntoResponse {
	let user_id = get_user(&jar,&state).await;
	
	if user_id.is_none(){
		return Err(Redirect::to("/login"))
	}
	
	let mut page = read_file_to_string(&PathBuf::from("templates/receipt_create.html")).await.unwrap();
	
	let payment_rows = sqlx::query("SELECT payment_type FROM payment_types")
		.fetch_all(&state.pool)
		.await
		.unwrap();
	
	let mut payment_html = String::new();
	
	for r in payment_rows {
		let p:String = r.get("payment_type");
		
		payment_html.push_str(
			&format!("<option value=\"{}\">{}</option>",p,p)
		);
	}
	
	let cashier_rows = sqlx::query("SELECT cashier_id,surname,first_name FROM cashiers")
		.fetch_all(&state.pool)
		.await
		.unwrap();
	
	let mut cashier_html = String::new();
	
	for r in cashier_rows {
		let id:i64 = r.get("cashier_id");
		let name:String = r.get("surname");
		let fname:String = r.get("first_name");
		
		cashier_html.push_str(
			&format!(
				"<option value=\"{}\">{} {}</option>",
				id,name,fname
			));
	}
	
	let delivery_rows = sqlx::query("SELECT delivery_type FROM delivery_types")
		.fetch_all(&state.pool)
		.await
		.unwrap();
	
	let mut delivery_html = String::new();
	
	for r in delivery_rows {
		let d:String = r.get("delivery_type");
		
		delivery_html.push_str(
			&format!("<option value=\"{}\">{}</option>",d,d)
		);
	}
	
	let store_rows = sqlx::query("SELECT store_id,address FROM stores")
		.fetch_all(&state.pool)
		.await
		.unwrap();
	
	let mut store_html = String::new();
	
	for r in store_rows {
		let id:i64 = r.get("store_id");
		let addr:String = r.get("address");
		
		store_html.push_str(
			&format!(
				"<option value=\"{}\">{}</option>",
				id,addr
			));
	}
	
	let goods_rows = sqlx::query("SELECT goods_id,name FROM goods ORDER BY name")
		.fetch_all(&state.pool)
		.await
		.unwrap();
	
	let mut goods_html = String::new();
	
	for r in goods_rows {
		let id:i64 = r.get("goods_id");
		let name:String = r.get("name");
		
		goods_html.push_str(
			&format!(
				"<option value=\"{}\">{}</option>",
				id,name
			));
	}
	
	let goods_rows = sqlx::query(
		"SELECT goods_id,name,price::float8 FROM goods ORDER BY name"
	)
		.fetch_all(&state.pool)
		.await
		.unwrap();
	
	let mut goods_js = String::new();
	
	for r in goods_rows {
		let id:i64 = r.get("goods_id");
		let name:String = r.get("name");
		let price:f64 = r.get("price");
		
		goods_js.push_str(
			&format!(
				"{{id:{},name:\"{}\",price:{}}},",
				id,name.replace("\"",""),price
			));
		
	}
	
	page = replace_in_html(page,"payment_types",&payment_html).await;
	page = replace_in_html(page,"cashiers",&cashier_html).await;
	page = replace_in_html(page,"delivery_types",&delivery_html).await;
	page = replace_in_html(page,"stores",&store_html).await;
	page = replace_in_html(page,"goods",&goods_html).await;
	page = replace_in_html(page,"goods_js", &goods_js).await;
	
	Ok(Html(page))
}