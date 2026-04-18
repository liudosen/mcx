use hmac::{Hmac, Mac};
use sha2::Sha256;

pub fn recharge_request_hash(secret: &str, openid: &str, amount: i64, payment_password: &str) -> String {
    let bucket = chrono::Utc::now().format("%Y%m%d").to_string();
    let mut mac = match <Hmac<Sha256> as Mac>::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => unreachable!("HMAC accepts any key length"),
    };
    mac.update(openid.as_bytes());
    mac.update(b"|");
    mac.update(amount.to_string().as_bytes());
    mac.update(b"|");
    mac.update(payment_password.as_bytes());
    mac.update(b"|");
    mac.update(bucket.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
