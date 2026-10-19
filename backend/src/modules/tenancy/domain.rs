use serde::{Deserialize, Serialize};

use crate::shared_kernel::TenantId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TenantStatus {
    Active,
    Suspended,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TenantProfile {
    pub id: TenantId,
    pub slug: String,
    pub name: String,
    pub status: TenantStatus,
}
