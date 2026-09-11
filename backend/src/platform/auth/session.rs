use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::shared_kernel::TenantId;

/// Shape persisted in Redis at `session:<hash-of-opaque-token>`.
///
/// The raw opaque token is never written to Redis or to logs — only its
/// SHA-256 hash is, mirroring how PAT-style credentials (GitHub, Stripe,
/// ...) are typically stored so a leaked snapshot of the session store
/// cannot be replayed directly as a bearer token.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionRecord {
    pub tenant_id: TenantId,
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub entitlements: Vec<String>,
}

pub fn hash_token(raw_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn session_key(token_hash: &str) -> String {
    format!("session:{token_hash}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashing_is_deterministic_and_not_the_identity_function() {
        let hash = hash_token("opaque-example-token");
        assert_eq!(hash, hash_token("opaque-example-token"));
        assert_ne!(hash, "opaque-example-token");
    }
}
