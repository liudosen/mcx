use crate::error::AppError;
use crate::models::order::status_label;
use crate::routes::admin::auth::authorize_admin;
use crate::routes::admin::permissions::DASHBOARD_VIEW;
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use std::sync::Arc;

const ORDER_STATUSES: [i8; 5] = [0, 1, 2, 3, 4];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DashboardQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub granularity: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub total_orders: i64,
    pub pending_orders: i64,
    pub shipping_orders: i64,
    pub completed_orders: i64,
    pub cancelled_orders: i64,
    pub today_orders: i64,
    pub today_revenue: i64,
    pub total_revenue: i64,
    pub total_users: i64,
    pub new_users_7d: i64,
    pub total_products: i64,
    pub active_products: i64,
    pub total_goods: i64,
    pub active_goods: i64,
    pub category_count: i64,
    pub low_stock_skus: i64,
    pub trend: Vec<TrendPoint>,
    pub status_breakdown: Vec<StatusCount>,
    pub top_products: Vec<TopProduct>,
    pub recent_orders: Vec<RecentOrder>,
    pub generated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    pub date: String,
    pub order_count: i64,
    pub paid_amount: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusCount {
    pub status: i8,
    pub status_label: String,
    pub count: i64,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TopProduct {
    pub id: u64,
    pub name: String,
    pub sales_count: i64,
    pub stock_quantity: i32,
    pub min_sale_price: i64,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RecentOrder {
    pub id: u64,
    pub order_no: String,
    pub status: i8,
    pub status_label: String,
    pub total_amount: i64,
    pub paid_amount: i64,
    pub discount_amount: i64,
    pub item_count: i64,
    pub customer_name: String,
    pub customer_phone: String,
    pub goods_summary: String,
    pub created_at: String,
}

#[derive(Debug, FromRow)]
struct TrendRow {
    period: String,
    order_count: i64,
    paid_amount: i64,
}

#[derive(Debug, FromRow)]
struct StatusRow {
    status: i8,
    count: i64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Granularity {
    Day,
    Week,
    Month,
}

impl Granularity {
    fn from_str(value: Option<&str>) -> Self {
        match value.unwrap_or("day") {
            "week" => Granularity::Week,
            "month" => Granularity::Month,
            _ => Granularity::Day,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Granularity::Day => "day",
            Granularity::Week => "week",
            Granularity::Month => "month",
        }
    }
}

fn parse_date(value: Option<&str>) -> Result<Option<NaiveDate>, AppError> {
    match value {
        Some(raw) if !raw.trim().is_empty() => NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
            .map(Some)
            .map_err(|_| AppError::BadRequest(format!("无效的日期格式: {}", raw))),
        _ => Ok(None),
    }
}

fn period_label(period: &str, granularity: Granularity) -> String {
    match granularity {
        Granularity::Day => period
            .get(5..)
            .map(|s| s.replace('-', "/"))
            .unwrap_or_else(|| period.to_string()),
        Granularity::Week => format!("{}周", period.replace('-', "/")),
        Granularity::Month => period.replace('-', "/"),
    }
}

pub async fn get_dashboard_data(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Query(query): Query<DashboardQuery>,
) -> Result<Json<ApiResponse<DashboardData>>, AppError> {
    authorize_admin(&state, &headers, &[DASHBOARD_VIEW]).await?;

    let end_date = parse_date(query.end_date.as_deref())?.unwrap_or_else(|| Local::now().date_naive());
    let start_date = parse_date(query.start_date.as_deref())?
        .unwrap_or_else(|| end_date - chrono::Duration::days(6));
    if start_date > end_date {
        return Err(AppError::BadRequest("开始日期不能晚于结束日期".to_string()));
    }

    let granularity = Granularity::from_str(query.granularity.as_deref());

    let total_orders = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM orders")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let pending_orders =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM orders WHERE status IN (0, 1)")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
    let shipping_orders = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM orders WHERE status = 1")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let completed_orders = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM orders WHERE status = 3")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let cancelled_orders = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM orders WHERE status = 4")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let today_orders = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM orders WHERE DATE(created_at) = CURRENT_DATE()",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);
    let today_revenue = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(paid_amount), 0) FROM orders WHERE DATE(created_at) = CURRENT_DATE()",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);
    let total_revenue = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(paid_amount), 0) FROM orders WHERE status IN (1, 2, 3)",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let total_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM wechat_users")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let new_users_7d = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM wechat_users WHERE created_at >= DATE_SUB(CURRENT_DATE(), INTERVAL 6 DAY)",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let total_products = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM products")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let active_products = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM products WHERE status = 1")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let total_goods = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM goods")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let active_goods = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM goods WHERE status = 1")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let category_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM goods_categories WHERE status = 1",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);
    let low_stock_skus = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM goods_skus WHERE stock_quantity <= 10",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let trend_sql = match granularity {
        Granularity::Day => {
            r#"
            SELECT DATE_FORMAT(created_at, '%Y-%m-%d') AS period,
                   COUNT(*) AS order_count,
                   COALESCE(SUM(paid_amount), 0) AS paid_amount
            FROM orders
            WHERE DATE(created_at) BETWEEN ? AND ?
            GROUP BY DATE_FORMAT(created_at, '%Y-%m-%d')
            ORDER BY period ASC
            "#
        }
        Granularity::Week => {
            r#"
            SELECT DATE_FORMAT(DATE_SUB(DATE(created_at), INTERVAL WEEKDAY(created_at) DAY), '%Y-%m-%d') AS period,
                   COUNT(*) AS order_count,
                   COALESCE(SUM(paid_amount), 0) AS paid_amount
            FROM orders
            WHERE DATE(created_at) BETWEEN ? AND ?
            GROUP BY DATE_FORMAT(DATE_SUB(DATE(created_at), INTERVAL WEEKDAY(created_at) DAY), '%Y-%m-%d')
            ORDER BY period ASC
            "#
        }
        Granularity::Month => {
            r#"
            SELECT DATE_FORMAT(created_at, '%Y-%m') AS period,
                   COUNT(*) AS order_count,
                   COALESCE(SUM(paid_amount), 0) AS paid_amount
            FROM orders
            WHERE DATE(created_at) BETWEEN ? AND ?
            GROUP BY DATE_FORMAT(created_at, '%Y-%m')
            ORDER BY period ASC
            "#
        }
    };

    let trend_rows: Vec<TrendRow> = sqlx::query_as(trend_sql)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    let trend = trend_rows
        .into_iter()
        .map(|row| TrendPoint {
            date: period_label(&row.period, granularity),
            order_count: row.order_count,
            paid_amount: row.paid_amount,
        })
        .collect::<Vec<_>>();

    let status_rows: Vec<StatusRow> = sqlx::query_as(
        "
        SELECT status, COUNT(*) AS count
        FROM orders
        WHERE DATE(created_at) BETWEEN ? AND ?
        GROUP BY status
        ",
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let status_map: HashMap<i8, i64> = status_rows.into_iter().map(|row| (row.status, row.count)).collect();
    let status_breakdown = ORDER_STATUSES
        .iter()
        .map(|status| StatusCount {
            status: *status,
            status_label: status_label(*status).to_string(),
            count: status_map.get(status).copied().unwrap_or(0),
        })
        .collect();

    let top_products = sqlx::query_as::<_, TopProduct>(
        r#"
        SELECT g.id,
               g.title AS name,
               COALESCE(SUM(CASE
                   WHEN DATE(o.created_at) BETWEEN ? AND ? THEN oi.quantity
                   ELSE 0
               END), 0) AS sales_count,
               g.spu_stock_quantity AS stock_quantity,
               g.min_sale_price
        FROM goods g
        LEFT JOIN order_items oi ON oi.spu_id = g.id
        LEFT JOIN orders o ON o.id = oi.order_id
        WHERE g.status = 1
        GROUP BY g.id, g.title, g.spu_stock_quantity, g.min_sale_price
        ORDER BY sales_count DESC, g.id DESC
        LIMIT 5
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let recent_orders = sqlx::query_as::<_, RecentOrder>(
        r#"
        SELECT o.id,
               o.order_no,
               o.status,
               CASE o.status
                   WHEN 0 THEN '待付款'
                   WHEN 1 THEN '待发货'
                   WHEN 2 THEN '待收货'
                   WHEN 3 THEN '已完成'
                   WHEN 4 THEN '已取消'
                   ELSE '未知'
               END AS status_label,
               o.total_amount,
               o.paid_amount,
               o.discount_amount,
               COALESCE((SELECT COUNT(*) FROM order_items oi WHERE oi.order_id = o.id), 0) AS item_count,
               COALESCE(u.real_name, '') AS customer_name,
               COALESCE(u.phone, '') AS customer_phone,
               COALESCE(
                   (SELECT GROUP_CONCAT(oi.goods_title ORDER BY oi.id SEPARATOR ' / ')
                    FROM order_items oi
                    WHERE oi.order_id = o.id),
                   ''
               ) AS goods_summary,
               DATE_FORMAT(o.created_at, '%Y-%m-%d %H:%i:%s') AS created_at
        FROM orders o
        LEFT JOIN wechat_users u ON u.id = o.user_id
        ORDER BY o.id DESC
        LIMIT 10
        "#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Ok(Json(ApiResponse::success(DashboardData {
        total_orders,
        pending_orders,
        shipping_orders,
        completed_orders,
        cancelled_orders,
        today_orders,
        today_revenue,
        total_revenue,
        total_users,
        new_users_7d,
        total_products,
        active_products,
        total_goods,
        active_goods,
        category_count,
        low_stock_skus,
        trend,
        status_breakdown,
        top_products,
        recent_orders,
        generated_at: format!(
            "{} ~ {} ({})",
            start_date.format("%Y-%m-%d"),
            end_date.format("%Y-%m-%d"),
            granularity.label()
        ),
    })))
}
