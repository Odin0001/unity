//! Shared Kernel: cross-cutting domain primitives shared by every bounded
//! context in `modules/`. Nothing in here may depend on `platform` or
//! `modules` — dependencies only ever point inward, toward this module.

pub mod money;
pub mod tenant_id;

pub use money::{Currency, Money, MoneyError};
pub use tenant_id::TenantId;
