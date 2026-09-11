use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts},
};
use redis::{aio::ConnectionManager, AsyncCommands};
use std::ops::Deref;

use super::{
    error::AuthError,
    session::{hash_token, session_key, SessionRecord},
    tenant_context::TenantContext,
};

/// Custom Axum extractor that authenticates a request via an opaque
/// bearer token.
///
/// Flow: read `Authorization: Bearer <token>` -> SHA-256 hash the raw
/// token (the raw token itself is never stored or logged) -> look up
/// `session:<hash>` in Redis -> deserialize the `SessionRecord` into a
/// `TenantContext`. That context is both returned to the handler as
/// `OpaqueAuth(ctx)` *and* inserted into `parts.extensions`, so it is
/// available to any extractor that runs later in the same handler's
/// argument list — notably `feature_flags::RequireModule<M>`.
///
/// `OpaqueAuth` must appear before `RequireModule<M>` in a handler's
/// argument list: Axum runs extractors in declaration order, and
/// `RequireModule` only reads `parts.extensions` — it never talks to
/// Redis itself.
pub struct OpaqueAuth(pub TenantContext);

impl Deref for OpaqueAuth {
    type Target = TenantContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequestParts<S> for OpaqueAuth
where
    S: Send + Sync,
    ConnectionManager: FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let raw_token = extract_bearer_token(parts)?;
        let token_hash = hash_token(raw_token);

        let mut redis = ConnectionManager::from_ref(state);
        let payload: Option<String> = redis
            .get(session_key(&token_hash))
            .await
            .map_err(|_| AuthError::SessionStoreUnavailable)?;

        let payload = payload.ok_or(AuthError::InvalidSession)?;
        let record: SessionRecord =
            serde_json::from_str(&payload).map_err(|_| AuthError::MalformedSession)?;

        let context = TenantContext {
            tenant_id: record.tenant_id,
            user_id: record.user_id,
            session_id: record.session_id,
            entitlements: record.entitlements,
        };

        parts.extensions.insert(context.clone());

        Ok(OpaqueAuth(context))
    }
}

fn extract_bearer_token(parts: &Parts) -> Result<&str, AuthError> {
    let header_value = parts
        .headers
        .get(header::AUTHORIZATION)
        .ok_or(AuthError::MissingCredentials)?;

    let header_str = header_value.to_str().map_err(|_| AuthError::InvalidScheme)?;
    let token = header_str
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidScheme)?
        .trim();

    if token.is_empty() {
        return Err(AuthError::InvalidScheme);
    }

    Ok(token)
}
