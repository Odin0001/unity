mod common;

use axum::http::StatusCode;
use common::{TestApp, EMPLOYEE_PASSWORD, EXECUTIVE_EMAIL};
use serde_json::json;
use sqlx::PgPool;
use tenants::modules::employees::application::create_executive_manager;

fn employees(create: bool, read: bool, update: bool, delete: bool) -> serde_json::Value {
    json!([{ "resource": "employees", "create": create, "read": read, "update": update, "delete": delete }])
}

async fn executive_id(app: &TestApp) -> String {
    let boss = app.executive_token().await;
    let me = app.send("GET", "/api/v1/auth/me", Some(&boss), None).await;
    me.body["id"].as_str().expect("executive id").to_string()
}

#[sqlx::test]
async fn first_employee_is_the_executive_with_full_access(db: PgPool) {
    let app = TestApp::new(db).await;
    let boss = app.executive_token().await;

    let me = app.send("GET", "/api/v1/auth/me", Some(&boss), None).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["email"], EXECUTIVE_EMAIL);
    assert_eq!(me.body["is_executive"], true);
    assert_eq!(me.body["permissions"], employees(true, true, true, true));
}

#[sqlx::test]
async fn executive_manager_is_only_created_into_an_empty_tenant(db: PgPool) {
    let app = TestApp::new(db).await;

    let again =
        create_executive_manager(&app.db, "other@acme.test", "another-password".into(), "X")
            .await
            .expect("no error");
    assert!(!again);

    let executives: i64 = sqlx::query_scalar("SELECT count(*) FROM employees WHERE is_executive")
        .fetch_one(&app.db)
        .await
        .expect("count");
    assert_eq!(executives, 1);
}

#[sqlx::test]
async fn new_employees_start_with_no_access(db: PgPool) {
    let app = TestApp::new(db).await;
    let (_, agent) = app.hire("agent@acme.test", json!([])).await;

    let me = app.send("GET", "/api/v1/auth/me", Some(&agent), None).await;
    assert_eq!(me.body["is_executive"], false);
    assert_eq!(me.body["permissions"], json!([]));

    let listed = app
        .send("GET", "/api/v1/employees", Some(&agent), None)
        .await;
    assert_eq!(listed.status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn each_action_needs_its_own_permission(db: PgPool) {
    let app = TestApp::new(db).await;
    let (_, reader) = app
        .hire("reader@acme.test", employees(false, true, false, false))
        .await;
    let boss_id = executive_id(&app).await;

    let listed = app
        .send("GET", "/api/v1/employees", Some(&reader), None)
        .await;
    assert_eq!(listed.status, StatusCode::OK);
    assert_eq!(listed.body.as_array().map(Vec::len), Some(2));

    let new_hire =
        json!({ "email": "x@acme.test", "full_name": "X", "password": EMPLOYEE_PASSWORD });
    let created = app
        .send(
            "POST",
            "/api/v1/employees",
            Some(&reader),
            Some(new_hire.clone()),
        )
        .await;
    assert_eq!(created.status, StatusCode::FORBIDDEN);

    let (_, creator) = app
        .hire("creator@acme.test", employees(true, false, false, false))
        .await;
    let created = app
        .send("POST", "/api/v1/employees", Some(&creator), Some(new_hire))
        .await;
    assert_eq!(created.status, StatusCode::CREATED);
    let new_id = created.body["id"].as_str().expect("id").to_string();

    let renamed = app
        .send(
            "PATCH",
            &format!("/api/v1/employees/{new_id}"),
            Some(&creator),
            Some(json!({ "full_name": "Y" })),
        )
        .await;
    assert_eq!(renamed.status, StatusCode::FORBIDDEN);

    let deleted = app
        .send(
            "DELETE",
            &format!("/api/v1/employees/{boss_id}"),
            Some(&reader),
            None,
        )
        .await;
    assert_eq!(deleted.status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn only_the_executive_sets_permissions(db: PgPool) {
    let app = TestApp::new(db).await;
    let (agent_id, manager) = app
        .hire("manager@acme.test", employees(true, true, true, true))
        .await;

    let attempt = app
        .send(
            "PUT",
            &format!("/api/v1/employees/{agent_id}/permissions"),
            Some(&manager),
            Some(employees(true, true, true, true)),
        )
        .await;
    assert_eq!(attempt.status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn executive_cannot_be_removed_or_deactivated(db: PgPool) {
    let app = TestApp::new(db).await;
    let boss = app.executive_token().await;
    let boss_id = executive_id(&app).await;
    let (_, manager) = app
        .hire("manager@acme.test", employees(true, true, true, true))
        .await;
    let uri = format!("/api/v1/employees/{boss_id}");

    let deleted = app.send("DELETE", &uri, Some(&manager), None).await;
    assert_eq!(deleted.status, StatusCode::FORBIDDEN);
    let renamed = app
        .send(
            "PATCH",
            &uri,
            Some(&manager),
            Some(json!({ "full_name": "Nope" })),
        )
        .await;
    assert_eq!(renamed.status, StatusCode::FORBIDDEN);

    let self_delete = app.send("DELETE", &uri, Some(&boss), None).await;
    assert_eq!(self_delete.status, StatusCode::FORBIDDEN);
    let self_deactivate = app
        .send(
            "PATCH",
            &uri,
            Some(&boss),
            Some(json!({ "is_active": false })),
        )
        .await;
    assert_eq!(self_deactivate.status, StatusCode::BAD_REQUEST);
    let self_rename = app
        .send(
            "PATCH",
            &uri,
            Some(&boss),
            Some(json!({ "full_name": "Big Boss" })),
        )
        .await;
    assert_eq!(self_rename.status, StatusCode::OK);
    assert_eq!(self_rename.body["full_name"], "Big Boss");
}

#[sqlx::test]
async fn permission_sets_are_validated(db: PgPool) {
    let app = TestApp::new(db).await;
    let boss = app.executive_token().await;
    let boss_id = executive_id(&app).await;
    let (agent_id, _) = app.hire("agent@acme.test", json!([])).await;

    let on_executive = app
        .send(
            "PUT",
            &format!("/api/v1/employees/{boss_id}/permissions"),
            Some(&boss),
            Some(json!([])),
        )
        .await;
    assert_eq!(on_executive.status, StatusCode::BAD_REQUEST);

    let uri = format!("/api/v1/employees/{agent_id}/permissions");
    let duplicated =
        json!([{ "resource": "employees", "read": true }, { "resource": "employees" }]);
    let response = app.send("PUT", &uri, Some(&boss), Some(duplicated)).await;
    assert_eq!(response.status, StatusCode::BAD_REQUEST);

    let unknown = json!([{ "resource": "spaceships", "read": true }]);
    let response = app.send("PUT", &uri, Some(&boss), Some(unknown)).await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);

    // Missing flags default to false.
    let partial = json!([{ "resource": "employees", "read": true }]);
    let response = app.send("PUT", &uri, Some(&boss), Some(partial)).await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body, employees(false, true, false, false));
}

#[sqlx::test]
async fn deactivated_and_deleted_employees_lose_access_immediately(db: PgPool) {
    let app = TestApp::new(db).await;
    let boss = app.executive_token().await;
    let (paused_id, paused) = app.hire("paused@acme.test", json!([])).await;
    let (gone_id, gone) = app.hire("gone@acme.test", json!([])).await;

    let deactivated = app
        .send(
            "PATCH",
            &format!("/api/v1/employees/{paused_id}"),
            Some(&boss),
            Some(json!({ "is_active": false })),
        )
        .await;
    assert_eq!(deactivated.status, StatusCode::OK);
    let me = app
        .send("GET", "/api/v1/auth/me", Some(&paused), None)
        .await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);
    let relogin = app.login("paused@acme.test", EMPLOYEE_PASSWORD).await;
    assert_eq!(relogin.status, StatusCode::UNAUTHORIZED);

    let uri = format!("/api/v1/employees/{gone_id}");
    assert_eq!(
        app.send("DELETE", &uri, Some(&boss), None).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.send("GET", &uri, Some(&boss), None).await.status,
        StatusCode::NOT_FOUND
    );
    let me = app.send("GET", "/api/v1/auth/me", Some(&gone), None).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn new_employee_input_is_checked(db: PgPool) {
    let app = TestApp::new(db).await;
    let boss = app.executive_token().await;
    let create = |email: &'static str, password: &'static str| {
        let body = json!({ "email": email, "full_name": "Agent", "password": password });
        app.send("POST", "/api/v1/employees", Some(&boss), Some(body))
    };

    assert_eq!(
        create("agent@acme.test", "short").await.status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        create("not-an-email", EMPLOYEE_PASSWORD).await.status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        create("Agent@Acme.test", EMPLOYEE_PASSWORD).await.status,
        StatusCode::CREATED
    );
    assert_eq!(
        create("agent@acme.test", EMPLOYEE_PASSWORD).await.status,
        StatusCode::CONFLICT
    );
}

#[sqlx::test]
async fn sign_in_and_out(db: PgPool) {
    let app = TestApp::new(db).await;

    let wrong = app.login(EXECUTIVE_EMAIL, "not-the-password").await;
    assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);

    let boss = app.executive_token().await;
    assert_eq!(boss.len(), 43);
    let out = app
        .send("POST", "/api/v1/auth/logout", Some(&boss), None)
        .await;
    assert_eq!(out.status, StatusCode::NO_CONTENT);
    let me = app.send("GET", "/api/v1/auth/me", Some(&boss), None).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);

    let anonymous = app.send("GET", "/api/v1/employees", None, None).await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
}
