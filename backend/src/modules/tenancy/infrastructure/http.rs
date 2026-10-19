use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;

use super::admin_client::AdminClientError;
use crate::{modules::tenancy::domain::TenantProfile, platform::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/tenant", get(current_tenant))
}

async fn current_tenant(
    State(state): State<AppState>,
) -> Result<Json<TenantProfile>, AdminClientError> {
    Ok(Json(state.admin.tenant_profile().await?))
}

impl IntoResponse for AdminClientError {
    fn into_response(self) -> Response {
        tracing::error!(error = %self, "admin service call failed");
        let body = Json(json!({ "error": "admin service unavailable" }));
        (StatusCode::BAD_GATEWAY, body).into_response()
    }
}
