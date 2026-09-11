use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared_kernel::TenantId;

/// Authenticated request context resolved from an opaque bearer token.
///
/// `OpaqueAuth` clones this into `http::Extensions` after a successful
/// Redis lookup so later extractors (e.g. `RequireModule`) can read it
/// without a second round trip to the session store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub entitlements: Vec<String>,
}

impl TenantContext {
    pub fn has_entitlement(&self, module: &str) -> bool {
        self.entitlements.iter().any(|entitlement| entitlement == module)
    }
}
