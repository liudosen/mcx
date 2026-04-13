use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    #[allow(dead_code)]
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub server_host: String,
    pub server_port: u16,
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

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Config {
            database_url: env::var("DATABASE_URL")?,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://default:liu5tgb^TFC@127.0.0.1:6379".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "default-secret-change-in-production".to_string()),
            jwt_expiry_hours: env::var("JWT_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8081".to_string())
                .parse()
                .unwrap_or(8081),
            bcrypt_cost: env::var("BCRYPT_COST")
                .unwrap_or_else(|_| "12".to_string())
                .parse()
                .unwrap_or(12),
            wechat_appid: env::var("WEIXIN_APPID")
                .unwrap_or_else(|_| "wx71cbe84503ce09ed".to_string()),
            wechat_secret: env::var("WEIXIN_SECRET").unwrap_or_else(|_| "".to_string()),
            jk_seller_username: env::var("JK_SELLER_USERNAME").unwrap_or_default(),
            jk_seller_password: env::var("JK_SELLER_PASSWORD").unwrap_or_default(),
            oss_endpoint: env::var("OSS_ENDPOINT")
                .unwrap_or_else(|_| "oss-cn-hangzhou.aliyuncs.com".to_string()),
            oss_access_key_id: env::var("OSS_ACCESS_KEY_ID").unwrap_or_default(),
            oss_access_key_secret: env::var("OSS_ACCESS_KEY_SECRET").unwrap_or_default(),
            oss_bucket: env::var("OSS_BUCKET").unwrap_or_else(|_| "welfare-store".to_string()),
            oss_domain: env::var("OSS_DOMAIN").unwrap_or_else(|_| {
                "https://welfare-store.oss-cn-hangzhou.aliyuncs.com".to_string()
            }),
        })
    }
}
