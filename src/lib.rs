//! figma-import — library surface, split out of `main.rs` purely so the
//! integration test (`tests/import_test.rs`) can drive the real router
//! with `tower::ServiceExt::oneshot` instead of spinning up a TCP
//! listener. Not meant to be depended on externally — this crate is a
//! deployable service, not an API.

pub mod error;
pub mod fig;
pub mod fixtures;

use axum::extract::{DefaultBodyLimit, Multipart};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use error::AppError;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Upstream `.fig` exports routinely carry hundreds of MB of embedded
/// bitmaps; 512 MiB keeps large real-world files working while still
/// bounding worst-case memory per request.
pub const MAX_UPLOAD_BYTES: usize = 512 * 1024 * 1024;

pub fn build_router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/import", post(import))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES))
        .layer(TraceLayer::new_for_http())
        .layer(
            // Internal service behind an app-tier caller (website-builder's
            // API server), not a browser origin directly — permissive CORS
            // is fine here and matches the sibling services' internal-only
            // network posture (see docker-compose.ai.yml: no host ports,
            // dedicated network).
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
}

/// `POST /import` — multipart upload, field name `file`. Optional `name`
/// text field overrides the document name (defaults to the uploaded
/// file's own filename, then `"Figma Import"`).
async fn import(mut multipart: Multipart) -> Result<impl IntoResponse, AppError> {
    let mut bytes: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut name_override: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::MultipartError(e.to_string()))?
    {
        match field.name().unwrap_or("") {
            "file" => {
                file_name = field.file_name().map(str::to_string);
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::MultipartError(e.to_string()))?;
                bytes = Some(data.to_vec());
            }
            "name" => {
                name_override = field.text().await.ok();
            }
            _ => {
                // Unknown field — ignore rather than reject, so a caller
                // can pass through extra form metadata without this
                // service needing to know about it.
            }
        }
    }

    let bytes = bytes.ok_or(AppError::MissingFile)?;
    let resolved_name = name_override
        .or(file_name)
        .unwrap_or_else(|| "Figma Import".to_string());

    let response = fig::import(&bytes, &resolved_name)?;
    Ok((StatusCode::OK, Json(response)))
}
