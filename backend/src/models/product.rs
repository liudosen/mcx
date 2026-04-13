use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: u64,
    pub name: String,
    pub description: Option<String>,
    pub price: i64,
    pub image_urls: String,
    pub category: Option<String>,
    pub status: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Product {
    #[allow(dead_code)]
    pub fn image_urls_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.image_urls).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub description: Option<String>,
    pub price: i64,
    #[serde(default)]
    pub image_urls: Vec<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<i64>,
    pub image_urls: Option<Vec<String>>,
    pub category: Option<String>,
    pub status: Option<bool>,
}
