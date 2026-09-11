pub mod error;
pub mod opaque_token;
pub mod session;
pub mod tenant_context;

// `error::AuthError` and `session::{SessionRecord, hash_token, session_key}`
// are reached through their owning submodule (`auth::error::AuthError`,
// `auth::session::hash_token`, ...) rather than re-exported here, since
// only `OpaqueAuth` and `TenantContext` are part of the surface other
// modules actually depend on today.
pub use opaque_token::OpaqueAuth;
pub use tenant_context::TenantContext;
