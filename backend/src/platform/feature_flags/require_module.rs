use axum::{extract::FromRequestParts, http::request::Parts};
use std::marker::PhantomData;

use super::error::FeatureFlagError;
use crate::platform::auth::TenantContext;

/// Compile-time module identity used to key entitlement checks.
///
/// The architecture calls for `RequireModule<const M: &'static str>`, but
/// a `&'static str` const generic parameter needs the still-unstable
/// `adt_const_params` nightly feature — not something to ship in a
/// production image. This trait + marker-type pattern is the stable-Rust
/// equivalent: each feature module defines its own zero-sized type
/// carrying its name as an associated const, e.g.:
///
/// ```ignore
/// pub struct ManufacturingModule;
/// impl Module for ManufacturingModule {
///     const NAME: &'static str = "manufacturing";
/// }
/// ```
///
/// which then gates a handler with `RequireModule<ManufacturingModule>` —
/// the same "module identity baked into the type" ergonomics the const
/// generic would have given, while compiling on stable.
pub trait Module {
    const NAME: &'static str;
}

/// Extractor that enforces module-level entitlement (the "paywall
/// checker"). Must run *after* `OpaqueAuth` in a handler's argument list:
/// it only reads the `TenantContext` `OpaqueAuth` already stashed in
/// `parts.extensions` and never talks to Redis or Postgres itself.
pub struct RequireModule<M: Module> {
    pub context: TenantContext,
    _module: PhantomData<M>,
}

impl<M: Module> RequireModule<M> {
    pub fn module_name(&self) -> &'static str {
        M::NAME
    }
}

impl<M, S> FromRequestParts<S> for RequireModule<M>
where
    M: Module + Send + Sync,
    S: Send + Sync,
{
    type Rejection = FeatureFlagError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let context = parts
            .extensions
            .get::<TenantContext>()
            .cloned()
            .ok_or(FeatureFlagError::MissingContext)?;

        if context.has_entitlement(M::NAME) {
            Ok(Self { context, _module: PhantomData })
        } else {
            Err(FeatureFlagError::ModuleNotEntitled(M::NAME))
        }
    }
}
