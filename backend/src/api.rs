use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    http::header,
    response::{IntoResponse, Json, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::storage::Storage;
use crate::types::{MessageDetail, MessageSummary};

pub fn app(storage: Storage) -> Router {
    Router::new()
        .route("/api/version", get(version))
        .route(
            "/api/messages",
            get(list_messages).delete(delete_all_messages),
        )
        .route(
            "/api/messages/{id}",
            get(get_message).delete(delete_message),
        )
        .route("/api/messages/{id}/raw", get(get_raw))
        .route("/api/messages/{id}/attachments/{aid}", get(get_attachment))
        .with_state(storage)
}

enum ApiError {
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
struct VersionResponse {
    version: &'static str,
}

async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn get_message(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> Result<Json<MessageDetail>, ApiError> {
    let message = storage.get_message(&id).await?;
    match message {
        Some(message) => Ok(Json(message)),
        None => Err(ApiError::NotFound("message not found".to_string())),
    }
}

#[derive(Deserialize)]
struct ListMessagesQuery {
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize)]
struct MessageListResponse {
    messages: Vec<MessageSummary>,
    total: i64,
    limit: i64,
    offset: i64,
}

async fn list_messages(
    State(storage): State<Storage>,
    Query(params): Query<ListMessagesQuery>,
) -> Result<Json<MessageListResponse>, ApiError> {
    let limit = match params.limit {
        Some(limit) if limit < 0 => {
            return Err(ApiError::BadRequest("limit must not be negative".to_string()));
        }
        Some(limit) => limit,
        None => 50,
    };

    let offset = match params.offset {
        Some(offset) if offset < 0 => {
            return Err(ApiError::BadRequest("offset must not be negative".to_string()));
        }
        Some(offset) => offset,
        None => 0,
    };

    let search = match &params.search {
        Some(search) => Some(search.as_str()),
        None => None,
    };

    let messages = storage.list_messages(search, limit, offset).await?;
    let total = storage.count_messages(search).await?;

    Ok(Json(MessageListResponse {
        messages,
        total,
        limit,
        offset,
    }))
}

async fn get_raw(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let raw_source = storage.get_raw_source(&id).await?;
    match raw_source {
        Some(raw_source) => {
            let headers = [
                (header::CONTENT_TYPE, "message/rfc822".to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{id}.eml\""),
                ),
            ];
            Ok((StatusCode::OK, headers, raw_source))
        }
        None => Err(ApiError::NotFound("message not found".to_string())),
    }
}

async fn get_attachment(
    State(storage): State<Storage>,
    Path((id, aid)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let attachment = storage.get_attachment(&id, &aid).await?;
    match attachment {
        Some(attachment) => {
            let content_type = match attachment.content_type {
                Some(content_type) => content_type,
                None => "application/octet-stream".to_string(),
            };
            let filename = match attachment.filename {
                Some(filename) => filename,
                None => "attachment".to_string(),
            };
            let headers = [
                (header::CONTENT_TYPE, content_type),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{filename}\""),
                ),
            ];
            Ok((StatusCode::OK, headers, attachment.data))
        }
        None => Err(ApiError::NotFound("attachment not found".to_string())),
    }
}

async fn delete_message(
    State(storage): State<Storage>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let deleted = storage.delete_message(&id).await?;
    match deleted {
        true => Ok(StatusCode::NO_CONTENT),
        false => Err(ApiError::NotFound("message not found".to_string())),
    }
}

async fn delete_all_messages(State(storage): State<Storage>) -> Result<StatusCode, ApiError> {
    storage.delete_all_messages().await?;
    Ok(StatusCode::NO_CONTENT)
}
