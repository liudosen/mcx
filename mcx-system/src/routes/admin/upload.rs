use crate::error::AppError;
use crate::routes::admin::auth::authorize_admin;
use crate::routes::admin::permissions::PRODUCT_LIST_VIEW;
use crate::routes::ApiResponse;
use crate::services::oss::UploadSignature;
use crate::state::AppState;
use axum::{extract::State, http::HeaderMap, Json};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct SignatureRequest {
    pub filename: String,
}

/// GET /api/admin/upload/signature
/// 获取 OSS 上传签名。
pub async fn get_upload_signature(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<SignatureRequest>,
) -> Result<Json<ApiResponse<UploadSignature>>, AppError> {
    authorize_admin(&state, &headers, &[PRODUCT_LIST_VIEW]).await?;

    let oss_service = crate::services::oss::OssService::new(
        state.oss_endpoint.clone(),
        state.oss_access_key_id.clone(),
        state.oss_access_key_secret.clone(),
        state.oss_bucket.clone(),
        state.oss_domain.clone(),
    );

    let signature = oss_service.generate_upload_signature(&params.filename)?;

    tracing::info!("Generated upload signature for file: {}", params.filename);

    Ok(Json(ApiResponse::success(signature)))
}
