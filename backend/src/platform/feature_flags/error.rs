use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FeatureFlagError {
    #[error("request has no authenticated tenant context; OpaqueAuth must run before RequireModule")]
    MissingContext,
    #[error("module '{0}' is not included in this tenant's current plan")]
    ModuleNotEntitled(&'static str),
}

impl IntoResponse for FeatureFlagError {
    fn into_response(self) -> Response {
        let status = match self {
            FeatureFlagError::MissingContext => StatusCode::UNAUTHORIZED,
            // 402 Payment Required communicates paywall intent precisely:
            // the caller is authenticated, just not entitled to this
            // module on their current plan.
            FeatureFlagError::ModuleNotEntitled(_) => StatusCode::PAYMENT_REQUIRED,
        };

        (status, Json(json!({ "error": self.to_string() }))).into_response()
    }
}
