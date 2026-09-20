use axum::{
    Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Json},
    routing::get,
};
use serde::Deserialize;

use crate::storage::Storage;

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

async fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

async fn get_message(State(storage): State<Storage>, Path(id): Path<String>) -> impl IntoResponse {
    match storage.get_message(&id).await {
        Ok(Some(message)) => (StatusCode::OK, Json(message)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(?e, "failed to fetch message");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize)]
struct ListMessagesQuery {
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn list_messages(
    State(storage): State<Storage>,
    Query(params): Query<ListMessagesQuery>,
) -> impl IntoResponse {
    let limit = match params.limit {
        Some(limit) if limit < 0 => {
            return (StatusCode::BAD_REQUEST, "limit must not be negative").into_response();
        }
        Some(limit) => limit,
        None => 50,
    };

    let offset = match params.offset {
        Some(offset) if offset < 0 => {
            return (StatusCode::BAD_REQUEST, "offset must not be negative").into_response();
        }
        Some(offset) => offset,
        None => 0,
    };

    let search = match &params.search {
        Some(search) => Some(search.as_str()),
        None => None,
    };

    match storage.list_messages(search, limit, offset).await {
        Ok(messages) => Json(messages).into_response(),
        Err(e) => {
            tracing::error!(?e, "failed to list messages");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn get_raw(State(storage): State<Storage>, Path(id): Path<String>) -> impl IntoResponse {
    match storage.get_raw_source(&id).await {
        Ok(Some(raw_source)) => {
            let headers = [
                (header::CONTENT_TYPE, "message/rfc822".to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{id}.eml\""),
                ),
            ];
            (StatusCode::OK, headers, raw_source).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(?e, "failed to fetch raw message");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn get_attachment(
    State(storage): State<Storage>,
    Path((id, aid)): Path<(String, String)>,
) -> impl IntoResponse {
    match storage.get_attachment(&id, &aid).await {
        Ok(Some(attachment)) => {
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
            (StatusCode::OK, headers, attachment.data).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(?e, "failed to fetch attachment");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn delete_message(State(storage): State<Storage>, Path(id): Path<String>) -> impl IntoResponse {
    match storage.delete_message(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(?e, "failed to delete message");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn delete_all_messages(State(storage): State<Storage>) -> impl IntoResponse {
    match storage.delete_all_messages().await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!(?e, "failed to delete all messages");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
