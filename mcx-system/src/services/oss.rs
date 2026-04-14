use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

#[derive(Clone)]
pub struct OssService {
    pub endpoint: String,
    pub access_key_id: String,
    pub access_key_secret: String,
    pub bucket: String,
    pub domain: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadSignature {
    pub url: String,
    pub key: String,
    pub policy: String,
    #[serde(rename = "OSSAccessKeyId")]
    pub oss_access_key_id: String,
    pub signature: String,
    pub expire: i64,
    pub host: String,
}

#[derive(Debug, Serialize)]
struct PolicyDocument {
    expiration: String,
    conditions: Vec<PolicyCondition>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum PolicyCondition {
    Exact { bucket: String },
    StartsWith(Vec<String>),
}

impl OssService {
    pub fn new(
        endpoint: String,
        access_key_id: String,
        access_key_secret: String,
        bucket: String,
        domain: String,
    ) -> Self {
        Self {
            endpoint,
            access_key_id,
            access_key_secret,
            bucket,
            domain,
        }
    }

    /// 生成上传签名（前端直传用）
    pub fn generate_upload_signature(&self, filename: &str) -> UploadSignature {
        let now = Utc::now();
        let expire_time = now + Duration::hours(1);

        // 生成文件路径：products/20260403/uuid_filename.jpg
        let date_path = now.format("%Y%m%d").to_string();
        let uuid = uuid::Uuid::new_v4().to_string();
        let key = format!("products/{}/{}_{}", date_path, uuid, filename);

        // 构建 Policy
        // OSS 要求时间格式为 ISO 8601，但不能包含纳秒
        let expiration = expire_time.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let policy = PolicyDocument {
            expiration,
            conditions: vec![
                PolicyCondition::Exact {
                    bucket: self.bucket.clone(),
                },
                PolicyCondition::StartsWith(vec![
                    "starts-with".to_string(),
                    "$key".to_string(),
                    "products/".to_string(),
                ]),
            ],
        };

        let policy_json = serde_json::to_string(&policy).unwrap();
        let policy_base64 = general_purpose::STANDARD.encode(&policy_json);

        // 生成签名
        let signature = self.sign(&policy_base64);

        let host = if self.domain.starts_with("http") {
            self.domain.clone()
        } else {
            format!("https://{}.{}", self.bucket, self.endpoint)
        };

        UploadSignature {
            url: host.clone(),
            key,
            policy: policy_base64,
            oss_access_key_id: self.access_key_id.clone(),
            signature,
            expire: expire_time.timestamp(),
            host,
        }
    }

    /// 生成签名
    fn sign(&self, string_to_sign: &str) -> String {
        let mut mac = HmacSha1::new_from_slice(self.access_key_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        let result = mac.finalize();
        general_purpose::STANDARD.encode(result.into_bytes())
    }

    /// 生成访问 URL（用于私有文件访问）
    #[allow(dead_code)]
    pub fn generate_presigned_url(&self, key: &str, expire_seconds: i64) -> String {
        let expire_timestamp = Utc::now().timestamp() + expire_seconds;

        let key_with_slash = if key.starts_with('/') {
            key.to_string()
        } else {
            format!("/{}", key)
        };

        let string_to_sign = format!(
            "GET\n\n\n{}\n/{}{}",
            expire_timestamp, self.bucket, key_with_slash
        );

        let signature = self.sign(&string_to_sign);
        let signature_encoded = urlencoding::encode(&signature);

        format!(
            "https://{}.{}{}?OSSAccessKeyId={}&Expires={}&Signature={}",
            self.bucket,
            self.endpoint,
            key_with_slash,
            self.access_key_id,
            expire_timestamp,
            signature_encoded
        )
    }
}
