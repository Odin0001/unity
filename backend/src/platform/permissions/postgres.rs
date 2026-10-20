use sqlx::PgPool;
use uuid::Uuid;

use super::Permission;

pub async fn granted_to(db: &PgPool, employee_id: Uuid) -> Result<Vec<Permission>, sqlx::Error> {
    sqlx::query_as(
        "SELECT resource, can_create, can_read, can_update, can_delete
         FROM employee_permissions
         WHERE employee_id = $1
         ORDER BY resource",
    )
    .bind(employee_id)
    .fetch_all(db)
    .await
}

pub async fn replace(
    db: &PgPool,
    employee_id: Uuid,
    permissions: &[Permission],
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;

    sqlx::query("DELETE FROM employee_permissions WHERE employee_id = $1")
        .bind(employee_id)
        .execute(&mut *tx)
        .await?;

    for p in permissions {
        sqlx::query(
            "INSERT INTO employee_permissions
                 (employee_id, resource, can_create, can_read, can_update, can_delete)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(employee_id)
        .bind(p.resource)
        .bind(p.create)
        .bind(p.read)
        .bind(p.update)
        .bind(p.delete)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await
}
