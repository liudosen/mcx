use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct AdminUser {
    pub id: u64,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub permission_codes: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessCodesResponse {
    pub username: String,
    pub role: String,
    pub is_admin: bool,
    pub codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionItem {
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGroup {
    pub name: String,
    pub items: Vec<PermissionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCatalogResponse {
    pub groups: Vec<PermissionGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AdminUserListItem {
    pub id: u64,
    pub username: String,
    pub role: String,
    pub permission_codes: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUserListResponse {
    pub list: Vec<AdminUserListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAdminUserPermissionsRequest {
    pub role: Option<String>,
    pub is_active: Option<bool>,
    pub permission_codes: Vec<String>,
}
