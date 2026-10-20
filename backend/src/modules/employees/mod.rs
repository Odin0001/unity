//! Employees of this tenant and their sign-in. The first employee is the
//! executive manager. Access checks go through `platform::permissions`.

pub mod application;
pub mod domain;
mod infrastructure;

pub use infrastructure::http::{auth_router, router, CurrentEmployee};
