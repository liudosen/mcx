use crate::error::AppError;
use crate::models::GoodsCategory;
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{extract::State, Json};
use std::sync::Arc;

pub async fn list_categories(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<GoodsCategory>>>, AppError> {
    let list = sqlx::query_as::<_, GoodsCategory>(
        "SELECT id, name, sort_order, status, (SELECT COUNT(*) FROM goods WHERE category_id = goods_categories.id) AS goods_count FROM goods_categories WHERE status = 1 ORDER BY sort_order ASC, id ASC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(list)))
}
