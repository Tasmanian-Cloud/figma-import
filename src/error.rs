//! Typed error surface for the import endpoint. Every failure mode is a
//! distinct variant with an HTTP status + machine-readable `code`, so a
//! caller (e.g. website-builder's import route) can branch on `code`
//! instead of pattern-matching a free-text message.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("no file field named `file` in the multipart upload")]
    MissingFile,

    #[error("uploaded file exceeds the {0} MiB limit")]
    TooLarge(u64),

    #[error(
        "unrecognised file format — not a binary .fig, a zip-wrapped .fig, or Figma clipboard JSON"
    )]
    UnknownFormat,

    #[error("Figma clipboard-JSON export (.fig.json) is not accepted by this endpoint; upload the binary .fig file")]
    ClipboardJsonNotSupported,

    #[error("failed to parse .fig binary: {0}")]
    ParseFailed(String),

    #[error("multipart read error: {0}")]
    MultipartError(String),
}

#[derive(Serialize)]
struct ErrorBody {
    ok: bool,
    code: &'static str,
    message: String,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            AppError::MissingFile => "missing_file",
            AppError::TooLarge(_) => "too_large",
            AppError::UnknownFormat => "unknown_format",
            AppError::ClipboardJsonNotSupported => "clipboard_json_not_supported",
            AppError::ParseFailed(_) => "parse_failed",
            AppError::MultipartError(_) => "multipart_error",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            AppError::MissingFile
            | AppError::UnknownFormat
            | AppError::ClipboardJsonNotSupported
            | AppError::ParseFailed(_)
            | AppError::MultipartError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::TooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = ErrorBody {
            ok: false,
            code: self.code(),
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}
