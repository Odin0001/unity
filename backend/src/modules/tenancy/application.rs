use super::{domain::TenantStatus, AdminClient};

/// Startup check. Admin may come up after us, so nothing here is fatal.
pub async fn check_admin_link(admin: &AdminClient) {
    match admin.tenant_profile().await {
        Ok(tenant) if tenant.status == TenantStatus::Active => {
            tracing::info!(tenant = %tenant.slug, "linked to admin service");
        }
        Ok(tenant) => tracing::warn!(tenant = %tenant.slug, "tenant is suspended"),
        Err(err) => tracing::warn!(%err, "admin service check failed"),
    }
}
