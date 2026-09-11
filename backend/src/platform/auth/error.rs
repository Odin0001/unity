use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("missing Authorization header")]
    MissingCredentials,
    #[error("invalid Authorization header, expected 'Bearer <opaque_token>'")]
    InvalidScheme,
    #[error("session not found or expired")]
    InvalidSession,
    #[error("session store unavailable")]
    SessionStoreUnavailable,
    #[error("stored session record could not be decoded")]
    MalformedSession,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = match self {
            AuthError::MissingCredentials | AuthError::InvalidScheme | AuthError::InvalidSession => {
                StatusCode::UNAUTHORIZED
            }
            AuthError::SessionStoreUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            AuthError::MalformedSession => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(json!({ "error": self.to_string() }))).into_response()
    }
}
