use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::employees::domain::{Employee, IDLE_TIMEOUT_MINS, SESSION_TTL_HOURS};

const COLUMNS: &str = "id, email, full_name, is_executive, is_active, created_at";

#[derive(sqlx::FromRow)]
struct Credentials {
    #[sqlx(flatten)]
    employee: Employee,
    password_hash: String,
}

pub async fn find_credentials(
    db: &PgPool,
    email: &str,
) -> Result<Option<(Employee, String)>, sqlx::Error> {
    let row: Option<Credentials> = sqlx::query_as(&format!(
        "SELECT {COLUMNS}, password_hash FROM employees WHERE email = $1 AND is_active"
    ))
    .bind(email)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|c| (c.employee, c.password_hash)))
}

pub async fn insert_session(
    db: &PgPool,
    token_hash: &str,
    employee_id: Uuid,
    user_agent: Option<&str>,
) -> Result<DateTime<Utc>, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO auth_sessions (token_hash, employee_id, user_agent, expires_at)
         VALUES ($1, $2, $3, now() + make_interval(hours => $4))
         RETURNING expires_at",
    )
    .bind(token_hash)
    .bind(employee_id)
    .bind(user_agent)
    .bind(SESSION_TTL_HOURS)
    .fetch_one(db)
    .await
}

/// Looks the session up and bumps `last_activity_at` in one round trip.
pub async fn touch_session(db: &PgPool, token_hash: &str) -> Result<Option<Employee>, sqlx::Error> {
    sqlx::query_as(
        "UPDATE auth_sessions s
         SET last_activity_at = now()
         FROM employees e
         WHERE s.token_hash = $1
           AND e.id = s.employee_id
           AND e.is_active
           AND s.expires_at > now()
           AND s.last_activity_at > now() - make_interval(mins => $2)
         RETURNING e.id, e.email, e.full_name, e.is_executive, e.is_active, e.created_at",
    )
    .bind(token_hash)
    .bind(IDLE_TIMEOUT_MINS)
    .fetch_optional(db)
    .await
}

pub async fn delete_session(db: &PgPool, token_hash: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM auth_sessions WHERE token_hash = $1")
        .bind(token_hash)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete_stale_sessions(db: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM auth_sessions
         WHERE expires_at <= now()
            OR last_activity_at <= now() - make_interval(mins => $1)",
    )
    .bind(IDLE_TIMEOUT_MINS)
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

pub async fn list(db: &PgPool) -> Result<Vec<Employee>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM employees ORDER BY created_at"
    ))
    .fetch_all(db)
    .await
}

pub async fn find(db: &PgPool, id: Uuid) -> Result<Option<Employee>, sqlx::Error> {
    sqlx::query_as(&format!("SELECT {COLUMNS} FROM employees WHERE id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn any_employees(db: &PgPool) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM employees)")
        .fetch_one(db)
        .await
}

/// `None` when the email is taken.
pub async fn insert(
    db: &PgPool,
    email: &str,
    password_hash: &str,
    full_name: &str,
) -> Result<Option<Employee>, sqlx::Error> {
    sqlx::query_as(&format!(
        "INSERT INTO employees (email, password_hash, full_name) VALUES ($1, $2, $3)
         ON CONFLICT (email) DO NOTHING
         RETURNING {COLUMNS}"
    ))
    .bind(email)
    .bind(password_hash)
    .bind(full_name)
    .fetch_optional(db)
    .await
}

/// Inserts the executive manager, but only into an empty table, so the
/// executive is always the first employee. `None` if anyone already exists.
pub async fn insert_first_executive(
    db: &PgPool,
    email: &str,
    password_hash: &str,
    full_name: &str,
) -> Result<Option<Employee>, sqlx::Error> {
    sqlx::query_as(&format!(
        "INSERT INTO employees (email, password_hash, full_name, is_executive)
         SELECT $1, $2, $3, true
         WHERE NOT EXISTS (SELECT 1 FROM employees)
         RETURNING {COLUMNS}"
    ))
    .bind(email)
    .bind(password_hash)
    .bind(full_name)
    .fetch_optional(db)
    .await
}

pub async fn update(
    db: &PgPool,
    id: Uuid,
    full_name: Option<&str>,
    is_active: Option<bool>,
) -> Result<Option<Employee>, sqlx::Error> {
    sqlx::query_as(&format!(
        "UPDATE employees
         SET full_name = COALESCE($2, full_name),
             is_active = COALESCE($3, is_active),
             updated_at = now()
         WHERE id = $1
         RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(full_name)
    .bind(is_active)
    .fetch_optional(db)
    .await
}

pub async fn delete(db: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM employees WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(result.rows_affected() == 1)
}
