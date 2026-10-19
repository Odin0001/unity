use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::platform::error::AppError;

pub const SESSION_TTL_HOURS: i32 = 12;
pub const IDLE_TIMEOUT_MINS: i32 = 60;
pub const MIN_PASSWORD_LEN: usize = 12;

/// Everything an employee can be given access to. New modules add their
/// resource here; the executive manager gets it automatically.
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

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Employee {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub is_executive: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// The signed-in employee behind a request, with what they're allowed to do.
#[derive(Debug, Clone, Serialize)]
pub struct Actor {
    #[serde(flatten)]
    pub employee: Employee,
    pub permissions: Vec<Permission>,
}

impl Actor {
    /// The executive manager can do anything; everyone else needs a matching
    /// permission row.
    ///
    /// ```
    /// use tenants::modules::employees::domain::{Action, Permission, Resource};
    /// # use tenants::modules::employees::domain::{Actor, Employee};
    /// # let employee = Employee {
    /// #     id: uuid::Uuid::new_v4(),
    /// #     email: "agent@acme.test".into(),
    /// #     full_name: "Agent".into(),
    /// #     is_executive: false,
    /// #     is_active: true,
    /// #     created_at: chrono::Utc::now(),
    /// # };
    /// let read_only = Permission {
    ///     resource: Resource::Employees,
    ///     create: false,
    ///     read: true,
    ///     update: false,
    ///     delete: false,
    /// };
    /// let actor = Actor { employee, permissions: vec![read_only] };
    ///
    /// assert!(actor.can(Resource::Employees, Action::Read));
    /// assert!(!actor.can(Resource::Employees, Action::Delete));
    /// ```
    pub fn can(&self, resource: Resource, action: Action) -> bool {
        self.employee.is_executive
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
        if self.employee.is_executive {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

/// What an employee can actually do: the stored rows, or everything for the
/// executive manager.
pub fn effective_permissions(employee: &Employee, stored: Vec<Permission>) -> Vec<Permission> {
    if employee.is_executive {
        Resource::ALL.into_iter().map(Permission::full).collect()
    } else {
        stored
    }
}

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn employee(is_executive: bool) -> Employee {
        Employee {
            id: Uuid::new_v4(),
            email: "someone@acme.test".into(),
            full_name: "Someone".into(),
            is_executive,
            is_active: true,
            created_at: Utc::now(),
        }
    }

    fn only(resource: Resource, action: Action) -> Permission {
        Permission {
            resource,
            create: action == Action::Create,
            read: action == Action::Read,
            update: action == Action::Update,
            delete: action == Action::Delete,
        }
    }

    #[test]
    fn executive_can_do_everything_without_any_rows() {
        let actor = Actor {
            employee: employee(true),
            permissions: vec![],
        };
        for action in [Action::Create, Action::Read, Action::Update, Action::Delete] {
            assert!(actor.can(Resource::Employees, action));
        }
        assert!(actor.require_executive().is_ok());
    }

    #[test]
    fn regular_employee_gets_exactly_what_was_granted() {
        let actor = Actor {
            employee: employee(false),
            permissions: vec![only(Resource::Employees, Action::Update)],
        };
        assert!(actor.can(Resource::Employees, Action::Update));
        assert!(!actor.can(Resource::Employees, Action::Read));
        assert!(matches!(
            actor.require(Resource::Employees, Action::Delete),
            Err(AppError::Forbidden)
        ));
        assert!(actor.require_executive().is_err());
    }

    #[test]
    fn effective_permissions_expand_only_for_the_executive() {
        let granted = vec![only(Resource::Employees, Action::Read)];
        assert_eq!(
            effective_permissions(&employee(false), granted.clone()),
            granted
        );
        assert_eq!(
            effective_permissions(&employee(true), vec![]),
            vec![Permission::full(Resource::Employees)]
        );
    }
}
