use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};

use crate::types::MessageSummary;

pub(super) enum ApiError {
    NotFound(String),
    BadRequest(String),
    Internal(anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(message) => (StatusCode::NOT_FOUND, message),
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::Internal(e) => {
                tracing::error!(?e, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        (status, Json(ErrorBody { error: message })).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError::Internal(e)
    }
}

#[derive(Serialize)]
pub(super) struct VersionResponse {
    pub(super) version: &'static str,
}

#[derive(Deserialize)]
pub(super) struct ListMessagesQuery {
    pub(super) search: Option<String>,
    pub(super) limit: Option<i64>,
    pub(super) offset: Option<i64>,
}

#[derive(Serialize)]
pub(super) struct MessageListResponse {
    pub(super) messages: Vec<MessageSummary>,
    pub(super) total: i64,
    pub(super) limit: i64,
    pub(super) offset: i64,
}
