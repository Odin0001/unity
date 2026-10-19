//! Employees of this tenant, their per-resource permissions, and sign-in.
//! The first employee is the executive manager and can do everything,
//! including deciding what everyone else can do.

pub mod application;
pub mod domain;
mod infrastructure;

pub use infrastructure::http::{auth_router, router, CurrentEmployee};
