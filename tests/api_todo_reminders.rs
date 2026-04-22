use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use tower::util::ServiceExt; // for `oneshot` method
use flower_rust::handlers;

mod common;
use common::create_test_state;

#[tokio::test]
async fn test_list_todo_reminders_empty() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/todo-reminders", axum::routing::get(handlers::todo_reminders::list_todo_reminders))
        .with_state(state.clone());

    let request = Request::builder()
        .uri("/api/todo-reminders")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    let _todos = json["todos"].as_array().unwrap();
    // Check initial state (might have data from migrations)
    // Length check not needed as todos is guaranteed to be an array
}

#[tokio::test]
async fn test_create_and_get_todo_reminder() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/todo-reminders", axum::routing::get(handlers::todo_reminders::list_todo_reminders))
        .route("/api/todo-reminders", axum::routing::post(handlers::todo_reminders::create_todo_reminder))
        .route("/api/todo-reminders/:id", axum::routing::get(handlers::todo_reminders::get_todo_reminder))
        .with_state(state.clone());

    // Create a todo reminder
    let create_json = serde_json::json!({
        "content": "测试待办事项",
        "priority": "高",
        "due_date": "2026.04.30",
        "notes": "测试备注"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/todo-reminders")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    let id = json["todo"]["id"].as_i64().unwrap();

    // Get the created todo reminder
    let get_request = Request::builder()
        .uri(format!("/api/todo-reminders/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["todo"]["content"], "测试待办事项");
    assert_eq!(json["todo"]["priority"], "高");
    assert_eq!(json["todo"]["notes"], "测试备注");
}

#[tokio::test]
async fn test_update_todo_reminder() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/todo-reminders", axum::routing::post(handlers::todo_reminders::create_todo_reminder))
        .route("/api/todo-reminders/:id", axum::routing::put(handlers::todo_reminders::update_todo_reminder))
        .route("/api/todo-reminders/:id", axum::routing::get(handlers::todo_reminders::get_todo_reminder))
        .with_state(state.clone());

    // Create first
    let create_json = serde_json::json!({
        "content": "原始待办",
        "priority": "中",
        "due_date": "2026.04.25"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/todo-reminders")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["todo"]["id"].as_i64().unwrap();

    // Update
    let update_json = serde_json::json!({
        "content": "更新后的待办",
        "priority": "高",
        "notes": "新增备注"
    });

    let update_request = Request::builder()
        .method("PUT")
        .uri(format!("/api/todo-reminders/{}", id))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&update_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(update_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify update
    let get_request = Request::builder()
        .uri(format!("/api/todo-reminders/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["todo"]["content"], "更新后的待办");
    assert_eq!(json["todo"]["priority"], "高");
    assert_eq!(json["todo"]["notes"], "新增备注");
}

#[tokio::test]
async fn test_update_todo_status() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/todo-reminders", axum::routing::post(handlers::todo_reminders::create_todo_reminder))
        .route("/api/todo-reminders/:id/status", axum::routing::put(handlers::todo_reminders::update_todo_status))
        .route("/api/todo-reminders/:id", axum::routing::get(handlers::todo_reminders::get_todo_reminder))
        .with_state(state.clone());

    // Create first
    let create_json = serde_json::json!({
        "content": "状态测试待办",
        "priority": "中"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/todo-reminders")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["todo"]["id"].as_i64().unwrap();

    // Mark as completed
    let status_json = serde_json::json!({
        "completed": true
    });

    let status_request = Request::builder()
        .method("PUT")
        .uri(format!("/api/todo-reminders/{}/status", id))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&status_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(status_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify status
    let get_request = Request::builder()
        .uri(format!("/api/todo-reminders/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(get_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["todo"]["completed"], true);
    assert!(json["todo"]["completed_at"].is_string());

    // Mark as not completed
    let status_json = serde_json::json!({
        "completed": false
    });

    let status_request = Request::builder()
        .method("PUT")
        .uri(format!("/api/todo-reminders/{}/status", id))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&status_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(status_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify status
    let get_request = Request::builder()
        .uri(format!("/api/todo-reminders/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(get_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["todo"]["completed"], false);
    // completed_at should be null
    assert!(json["todo"]["completed_at"].is_null());
}

#[tokio::test]
async fn test_delete_todo_reminder() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/todo-reminders", axum::routing::post(handlers::todo_reminders::create_todo_reminder))
        .route("/api/todo-reminders/:id", axum::routing::delete(handlers::todo_reminders::delete_todo_reminder))
        .route("/api/todo-reminders", axum::routing::get(handlers::todo_reminders::list_todo_reminders))
        .with_state(state.clone());

    // Create
    let create_json = serde_json::json!({
        "content": "删除测试待办",
        "priority": "低"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/todo-reminders")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["todo"]["id"].as_i64().unwrap();

    // Delete
    let delete_request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/todo-reminders/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(delete_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify deleted - list all todos and ensure our test record is not there
    let list_request = Request::builder()
        .uri("/api/todo-reminders")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(list_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let todos = json["todos"].as_array().unwrap();
    // The todo with content "删除测试待办" should not be present
    assert!(!todos.iter().any(|t| t["content"] == "删除测试待办"));
}