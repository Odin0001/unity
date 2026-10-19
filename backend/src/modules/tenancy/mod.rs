//! Which tenant this deployment is, as the platform admin sees it.

pub mod application;
pub mod domain;
mod infrastructure;

pub use infrastructure::{
    admin_client::{AdminClient, AdminClientError},
    http::router,
};
