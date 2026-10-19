use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    domain::{
        effective_permissions, normalize_email, Action, Actor, Employee, Permission, Resource,
        MIN_PASSWORD_LEN,
    },
    infrastructure::postgres,
};
use crate::platform::{error::AppError, password, token};

#[derive(Debug, Serialize)]
pub struct Session {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub employee: Actor,
}

pub async fn login(
    db: &PgPool,
    email: &str,
    password: String,
    user_agent: Option<&str>,
) -> Result<Session, AppError> {
    let (employee, password_hash) = postgres::find_credentials(db, &normalize_email(email))
        .await?
        .ok_or(AppError::InvalidCredentials)?;

    if !password::verify(password, password_hash).await? {
        return Err(AppError::InvalidCredentials);
    }

    let token = token::generate();
    let expires_at =
        postgres::insert_session(db, &token::hash(&token), employee.id, user_agent).await?;

    Ok(Session {
        token,
        expires_at,
        employee: with_permissions(db, employee).await?,
    })
}

/// Fails for unknown, expired or idle sessions, and for employees who were
/// deactivated after signing in.
pub async fn authenticate(db: &PgPool, token: &str) -> Result<Actor, AppError> {
    let employee = postgres::touch_session(db, &token::hash(token))
        .await?
        .ok_or(AppError::Unauthorized)?;
    with_permissions(db, employee).await
}

pub async fn logout(db: &PgPool, token: &str) -> Result<(), AppError> {
    postgres::delete_session(db, &token::hash(token)).await?;
    Ok(())
}

pub async fn purge_expired_sessions(db: &PgPool) -> Result<u64, AppError> {
    Ok(postgres::delete_stale_sessions(db).await?)
}

/// Startup hook: creates the executive manager from `EXECUTIVE_MANAGER_EMAIL`,
/// `EXECUTIVE_MANAGER_PASSWORD` and optionally `EXECUTIVE_MANAGER_NAME`.
pub async fn ensure_executive_manager(db: &PgPool) -> Result<(), AppError> {
    // compose turns unset variables into empty strings
    let var = |name| std::env::var(name).ok().filter(|v| !v.is_empty());
    let (Some(email), Some(password)) = (
        var("EXECUTIVE_MANAGER_EMAIL"),
        var("EXECUTIVE_MANAGER_PASSWORD"),
    ) else {
        if !postgres::any_employees(db).await? {
            tracing::warn!(
                "no employees yet; set EXECUTIVE_MANAGER_EMAIL and EXECUTIVE_MANAGER_PASSWORD"
            );
        }
        return Ok(());
    };
    let name = var("EXECUTIVE_MANAGER_NAME").unwrap_or_else(|| "Executive Manager".into());

    if create_executive_manager(db, &email, password, &name).await? {
        tracing::info!(%email, "created executive manager");
    }
    Ok(())
}

/// Makes the first employee, who becomes the executive manager. Returns
/// `false` and changes nothing once the tenant has any employee at all.
pub async fn create_executive_manager(
    db: &PgPool,
    email: &str,
    password: String,
    full_name: &str,
) -> Result<bool, AppError> {
    if postgres::any_employees(db).await? {
        return Ok(false);
    }
    let (email, full_name) = validate_new_employee(email, full_name, &password)?;
    let password_hash = password::hash(password).await?;

    let created = postgres::insert_first_executive(db, &email, &password_hash, full_name).await?;
    Ok(created.is_some())
}

pub async fn list(db: &PgPool, actor: &Actor) -> Result<Vec<Employee>, AppError> {
    actor.require(Resource::Employees, Action::Read)?;
    Ok(postgres::list(db).await?)
}

pub async fn get(db: &PgPool, actor: &Actor, id: Uuid) -> Result<Actor, AppError> {
    actor.require(Resource::Employees, Action::Read)?;
    let employee = postgres::find(db, id).await?.ok_or(AppError::NotFound)?;
    with_permissions(db, employee).await
}

/// New employees start with no permissions; the executive manager grants
/// them afterwards.
pub async fn create(
    db: &PgPool,
    actor: &Actor,
    email: &str,
    full_name: &str,
    password: String,
) -> Result<Employee, AppError> {
    actor.require(Resource::Employees, Action::Create)?;
    let (email, full_name) = validate_new_employee(email, full_name, &password)?;
    let password_hash = password::hash(password).await?;

    let employee = postgres::insert(db, &email, &password_hash, full_name)
        .await?
        .ok_or(AppError::Conflict(
            "an employee with this email already exists",
        ))?;

    tracing::info!(employee_id = %employee.id, created_by = %actor.employee.id, "employee created");
    Ok(employee)
}

pub async fn update(
    db: &PgPool,
    actor: &Actor,
    id: Uuid,
    full_name: Option<&str>,
    is_active: Option<bool>,
) -> Result<Employee, AppError> {
    actor.require(Resource::Employees, Action::Update)?;
    let target = postgres::find(db, id).await?.ok_or(AppError::NotFound)?;

    if target.is_executive {
        // Only the executive edits their own record, and they stay active.
        actor.require_executive()?;
        if is_active == Some(false) {
            return Err(AppError::Invalid(
                "the executive manager can't be deactivated",
            ));
        }
    }

    let full_name = full_name.map(str::trim);
    if full_name == Some("") {
        return Err(AppError::Invalid("full_name can't be empty"));
    }

    postgres::update(db, id, full_name, is_active)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn delete(db: &PgPool, actor: &Actor, id: Uuid) -> Result<(), AppError> {
    actor.require(Resource::Employees, Action::Delete)?;
    let target = postgres::find(db, id).await?.ok_or(AppError::NotFound)?;
    if target.is_executive {
        return Err(AppError::Forbidden);
    }

    postgres::delete(db, id).await?;
    tracing::info!(employee_id = %id, deleted_by = %actor.employee.id, "employee deleted");
    Ok(())
}

/// Replaces an employee's permissions. Only the executive manager decides
/// who can do what.
pub async fn set_permissions(
    db: &PgPool,
    actor: &Actor,
    id: Uuid,
    permissions: Vec<Permission>,
) -> Result<Vec<Permission>, AppError> {
    actor.require_executive()?;
    let target = postgres::find(db, id).await?.ok_or(AppError::NotFound)?;
    if target.is_executive {
        return Err(AppError::Invalid(
            "the executive manager always has full access",
        ));
    }

    let mut seen = Vec::with_capacity(permissions.len());
    for p in &permissions {
        if seen.contains(&p.resource) {
            return Err(AppError::Invalid("each resource can only appear once"));
        }
        seen.push(p.resource);
    }

    postgres::replace_permissions(db, id, &permissions).await?;
    Ok(postgres::permissions_of(db, id).await?)
}

async fn with_permissions(db: &PgPool, employee: Employee) -> Result<Actor, AppError> {
    let stored = if employee.is_executive {
        Vec::new()
    } else {
        postgres::permissions_of(db, employee.id).await?
    };
    Ok(Actor {
        permissions: effective_permissions(&employee, stored),
        employee,
    })
}

fn validate_new_employee<'a>(
    email: &str,
    full_name: &'a str,
    password: &str,
) -> Result<(String, &'a str), AppError> {
    let email = normalize_email(email);
    if !email.contains('@') {
        return Err(AppError::Invalid("email is not valid"));
    }
    let full_name = full_name.trim();
    if full_name.is_empty() {
        return Err(AppError::Invalid("full_name is required"));
    }
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(AppError::Invalid("password must be at least 12 characters"));
    }
    Ok((email, full_name))
}
