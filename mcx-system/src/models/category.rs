use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct GoodsCategory {
    pub id: u64,
    pub name: String,
    pub sort_order: i32,
    pub status: bool,
    pub goods_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub sort_order: Option<i32>,
    pub status: Option<bool>,
}
