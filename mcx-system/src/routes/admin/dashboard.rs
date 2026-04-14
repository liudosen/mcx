use crate::error::AppError;
use crate::routes::admin::auth::check_admin;
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::FromRow;
use std::sync::Arc;

/// 监控大屏数据
#[derive(Debug, Serialize)]
pub struct DashboardData {
    pub total_orders: i64,
    pub logistics_count: i64,
    pub total_users: i64,
    pub total_products: i64,
    pub top_products: Vec<TopProduct>,
}

/// 商品排名
#[derive(Debug, Serialize, FromRow)]
pub struct TopProduct {
    pub id: u64,
    pub name: String,
    pub sales_count: i64,
}

/// 获取监控大屏数据
pub async fn get_dashboard_data(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<DashboardData>>, AppError> {
    check_admin(&state, &headers).await?;

    // 总订单数（订单表暂未创建，展示0）
    let total_orders: i64 = 0;

    // 物流中数量（订单表暂未创建，展示0）
    let logistics_count: i64 = 0;

    // 用户总数
    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wechat_users")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    // 商品总数
    let total_products: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE status = 1")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    // 热销商品排名（订单表暂未创建，展示热门商品）
    let top_products_sql = r#"
        SELECT id, name, 0 as sales_count
        FROM products
        WHERE status = 1
        ORDER BY id DESC
        LIMIT 5
    "#;
    let top_products: Vec<TopProduct> = sqlx::query_as(top_products_sql)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    Ok(Json(ApiResponse::success(DashboardData {
        total_orders,
        logistics_count,
        total_users,
        total_products,
        top_products,
    })))
}
