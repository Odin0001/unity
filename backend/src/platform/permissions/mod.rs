//! Who may do what. Every module checks access through here. Permissions are
//! create / read / update / delete flags per resource, granted per employee;
//! the executive manager passes every check without any stored rows.

mod postgres;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::error::AppError;

/// Everything access can be granted on. A new module adds its resource here
/// and the executive manager gets it automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    Employees,
}

impl Resource {
    pub const ALL: [Resource; 1] = [Resource::Employees];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Create,
    Read,
    Update,
    Delete,
}

/// Flags left out of a request default to `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct Permission {
    pub resource: Resource,
    #[sqlx(rename = "can_create")]
    #[serde(default)]
    pub create: bool,
    #[sqlx(rename = "can_read")]
    #[serde(default)]
    pub read: bool,
    #[sqlx(rename = "can_update")]
    #[serde(default)]
    pub update: bool,
    #[sqlx(rename = "can_delete")]
    #[serde(default)]
    pub delete: bool,
}

impl Permission {
    pub fn full(resource: Resource) -> Self {
        Self {
            resource,
            create: true,
            read: true,
            update: true,
            delete: true,
        }
    }

    pub fn allows(&self, action: Action) -> bool {
        match action {
            Action::Create => self.create,
            Action::Read => self.read,
            Action::Update => self.update,
            Action::Delete => self.delete,
        }
    }
}

/// What one employee is allowed to do. Serializes as the list of their
/// effective permissions.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct Access {
    #[serde(skip)]
    is_executive: bool,
    permissions: Vec<Permission>,
}

impl Access {
    /// The executive manager gets full access to every resource; anyone else
    /// gets exactly what was granted.
    ///
    /// ```
    /// use tenants::platform::permissions::{Access, Action, Permission, Resource};
    ///
    /// let read_only = Permission {
    ///     resource: Resource::Employees,
    ///     create: false,
    ///     read: true,
    ///     update: false,
    ///     delete: false,
    /// };
    /// let agent = Access::new(false, vec![read_only]);
    /// assert!(agent.can(Resource::Employees, Action::Read));
    /// assert!(!agent.can(Resource::Employees, Action::Delete));
    ///
    /// let boss = Access::new(true, vec![]);
    /// assert!(boss.can(Resource::Employees, Action::Delete));
    /// ```
    pub fn new(is_executive: bool, granted: Vec<Permission>) -> Self {
        let permissions = if is_executive {
            Resource::ALL.into_iter().map(Permission::full).collect()
        } else {
            granted
        };
        Self {
            is_executive,
            permissions,
        }
    }

    pub fn can(&self, resource: Resource, action: Action) -> bool {
        self.is_executive
            || self
                .permissions
                .iter()
                .any(|p| p.resource == resource && p.allows(action))
    }

    pub fn require(&self, resource: Resource, action: Action) -> Result<(), AppError> {
        if self.can(resource, action) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    pub fn require_executive(&self) -> Result<(), AppError> {
        if self.is_executive {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    pub fn permissions(&self) -> &[Permission] {
        &self.permissions
    }
}

/// Loads an employee's access. The executive's is known without a query.
pub async fn load(db: &PgPool, employee_id: Uuid, is_executive: bool) -> Result<Access, AppError> {
    let granted = if is_executive {
        Vec::new()
    } else {
        postgres::granted_to(db, employee_id).await?
    };
    Ok(Access::new(is_executive, granted))
}

/// Replaces everything granted to an employee and returns what's stored.
/// Each resource may appear once.
pub async fn replace(
    db: &PgPool,
    employee_id: Uuid,
    permissions: &[Permission],
) -> Result<Vec<Permission>, AppError> {
    for (i, p) in permissions.iter().enumerate() {
        if permissions[..i].iter().any(|q| q.resource == p.resource) {
            return Err(AppError::Invalid("each resource can only appear once"));
        }
    }

    postgres::replace(db, employee_id, permissions).await?;
    Ok(postgres::granted_to(db, employee_id).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn only(action: Action) -> Permission {
        Permission {
            resource: Resource::Employees,
            create: action == Action::Create,
            read: action == Action::Read,
            update: action == Action::Update,
            delete: action == Action::Delete,
        }
    }

    #[test]
    fn executive_can_do_everything_without_any_rows() {
        let access = Access::new(true, vec![]);
        for action in [Action::Create, Action::Read, Action::Update, Action::Delete] {
            assert!(access.can(Resource::Employees, action));
        }
        assert!(access.require_executive().is_ok());
        assert_eq!(
            access.permissions(),
            [Permission::full(Resource::Employees)]
        );
    }

    #[test]
    fn others_get_exactly_what_was_granted() {
        let access = Access::new(false, vec![only(Action::Update)]);
        assert!(access.can(Resource::Employees, Action::Update));
        assert!(!access.can(Resource::Employees, Action::Read));
        assert!(matches!(
            access.require(Resource::Employees, Action::Delete),
            Err(AppError::Forbidden)
        ));
        assert!(access.require_executive().is_err());
        assert_eq!(access.permissions(), [only(Action::Update)]);
    }

    #[test]
    fn serializes_as_the_permission_list() -> Result<(), serde_json::Error> {
        let json = serde_json::to_value(Access::new(false, vec![only(Action::Read)]))?;
        assert_eq!(
            json,
            serde_json::json!([{
                "resource": "employees",
                "create": false,
                "read": true,
                "update": false,
                "delete": false,
            }])
        );
        Ok(())
    }
}
