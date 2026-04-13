use redis::aio::ConnectionManager;
use sqlx::MySqlPool;

#[derive(Clone)]
pub struct AppState {
    pub db: MySqlPool,
    pub redis: ConnectionManager,
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    #[allow(dead_code)]
    pub bcrypt_cost: u32,
    pub wechat_appid: String,
    pub wechat_secret: String,
    pub jk_seller_username: String,
    pub jk_seller_password: String,
    pub oss_endpoint: String,
    pub oss_access_key_id: String,
    pub oss_access_key_secret: String,
    pub oss_bucket: String,
    pub oss_domain: String,
}
