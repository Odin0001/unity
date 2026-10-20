use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

pub const SESSION_TTL_HOURS: i32 = 12;
pub const IDLE_TIMEOUT_MINS: i32 = 60;
pub const MIN_PASSWORD_LEN: usize = 12;

/// An employee of this tenant. What they may do lives in
/// `platform::permissions`, not here.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Employee {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    /// The first employee: full access everywhere, and the only one who
    /// grants permissions to others.
    pub is_executive: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// ```
/// use tenants::modules::employees::domain::normalize_email;
///
/// assert_eq!(normalize_email("  Jane@Acme.TEST "), "jane@acme.test");
/// ```
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}
