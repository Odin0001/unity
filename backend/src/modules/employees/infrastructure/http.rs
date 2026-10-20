use axum::{
    extract::{FromRequestParts, Path, State},
    http::{header, request::Parts, HeaderMap, StatusCode},
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    modules::employees::{
        application::{self, Actor, Session},
        domain::Employee,
    },
    platform::{error::AppError, permissions::Permission, AppState},
};

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(show).patch(update).delete(remove))
        .route("/{id}/permissions", put(set_permissions))
}

/// Taking this as a handler argument makes the route require a signed-in
/// employee (`Authorization: Bearer <token>`).
pub struct CurrentEmployee(pub Actor);

impl FromRequestParts<AppState> for CurrentEmployee {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let token = bearer(&parts.headers).ok_or(AppError::Unauthorized)?;
        application::authenticate(&state.db, token).await.map(Self)
    }
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Json<Session>, AppError> {
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok());

    let session = application::login(&state.db, &req.email, req.password, user_agent).await?;
    tracing::info!(employee_id = %session.employee.employee.id, "employee signed in");
    Ok(Json(session))
}

// Unknown or already revoked tokens still get a 204.
async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<StatusCode, AppError> {
    let token = bearer(&headers).ok_or(AppError::Unauthorized)?;
    application::logout(&state.db, token).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn me(CurrentEmployee(actor): CurrentEmployee) -> Json<Actor> {
    Json(actor)
}

async fn list(
    State(state): State<AppState>,
    CurrentEmployee(actor): CurrentEmployee,
) -> Result<Json<Vec<Employee>>, AppError> {
    Ok(Json(application::list(&state.db, &actor).await?))
}

async fn show(
    State(state): State<AppState>,
    CurrentEmployee(actor): CurrentEmployee,
    Path(id): Path<Uuid>,
) -> Result<Json<Actor>, AppError> {
    Ok(Json(application::get(&state.db, &actor, id).await?))
}

#[derive(Deserialize)]
struct CreateEmployee {
    email: String,
    full_name: String,
    password: String,
}

async fn create(
    State(state): State<AppState>,
    CurrentEmployee(actor): CurrentEmployee,
    Json(req): Json<CreateEmployee>,
) -> Result<(StatusCode, Json<Employee>), AppError> {
    let employee =
        application::create(&state.db, &actor, &req.email, &req.full_name, req.password).await?;
    Ok((StatusCode::CREATED, Json(employee)))
}

#[derive(Deserialize)]
struct UpdateEmployee {
    full_name: Option<String>,
    is_active: Option<bool>,
}

async fn update(
    State(state): State<AppState>,
    CurrentEmployee(actor): CurrentEmployee,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateEmployee>,
) -> Result<Json<Employee>, AppError> {
    let employee = application::update(
        &state.db,
        &actor,
        id,
        req.full_name.as_deref(),
        req.is_active,
    )
    .await?;
    Ok(Json(employee))
}

async fn remove(
    State(state): State<AppState>,
    CurrentEmployee(actor): CurrentEmployee,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    application::delete(&state.db, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn set_permissions(
    State(state): State<AppState>,
    CurrentEmployee(actor): CurrentEmployee,
    Path(id): Path<Uuid>,
    Json(permissions): Json<Vec<Permission>>,
) -> Result<Json<Vec<Permission>>, AppError> {
    Ok(Json(
        application::set_permissions(&state.db, &actor, id, &permissions).await?,
    ))
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn auth(value: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static(value));
        headers
    }

    #[test]
    fn reads_bearer_token() {
        assert_eq!(bearer(&auth("Bearer abc")), Some("abc"));
    }

    #[test]
    fn ignores_other_schemes_and_blank_tokens() {
        assert_eq!(bearer(&HeaderMap::new()), None);
        assert_eq!(bearer(&auth("Basic abc")), None);
        assert_eq!(bearer(&auth("Bearer    ")), None);
    }
}
