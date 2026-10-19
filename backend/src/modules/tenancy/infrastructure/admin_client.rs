use std::time::Duration;

use crate::modules::tenancy::domain::TenantProfile;

#[derive(Debug, thiserror::Error)]
pub enum AdminClientError {
    #[error("{0} is not set")]
    MissingConfig(&'static str),
    #[error("admin service request failed: {0}")]
    Http(#[from] reqwest::Error),
}

/// Talks to the platform admin, authenticated with this tenant's API key
/// (`X-Tenant-Key`).
///
/// ```no_run
/// use tenants::modules::tenancy::{AdminClient, AdminClientError};
///
/// # async fn demo() -> Result<(), AdminClientError> {
/// let admin = AdminClient::new("http://localhost:8081", "tenant-api-key".into())?;
/// let me = admin.tenant_profile().await?;
/// println!("{} is {:?}", me.name, me.status);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct AdminClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl AdminClient {
    pub fn new(base_url: &str, api_key: String) -> Result<Self, AdminClientError> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    /// Reads `ADMIN_API_URL` and `TENANT_API_KEY`. Empty counts as unset.
    pub fn from_env() -> Result<Self, AdminClientError> {
        let env = |name| {
            std::env::var(name)
                .ok()
                .filter(|v| !v.is_empty())
                .ok_or(AdminClientError::MissingConfig(name))
        };

        Self::new(&env("ADMIN_API_URL")?, env("TENANT_API_KEY")?)
    }

    pub async fn tenant_profile(&self) -> Result<TenantProfile, AdminClientError> {
        let profile = self
            .http
            .get(format!("{}/api/v1/internal/tenant", self.base_url))
            .header("x-tenant-key", &self.api_key)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(profile)
    }
}
