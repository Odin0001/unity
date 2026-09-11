use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

/// Opaque tenant identifier.
///
/// Wrapping `Uuid` in a distinct newtype stops a stray `Uuid` (a user id,
/// an item id, ...) from ever being passed where a tenant id is expected —
/// the compiler enforces a distinction that would otherwise only live in a
/// variable name. `#[sqlx(transparent)]` lets it bind/decode directly as a
/// Postgres `uuid` column with no manual `Encode`/`Decode` boilerplate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct TenantId(Uuid);

impl TenantId {
    pub fn new(id: Uuid) -> Self {
        Self(id)
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TenantId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for TenantId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_string() {
        let id = TenantId::generate();
        let parsed: TenantId = id.to_string().parse().expect("valid uuid string");
        assert_eq!(id, parsed);
    }
}
