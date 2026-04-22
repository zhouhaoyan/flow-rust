use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use tower::util::ServiceExt; // for `oneshot` method
use flower_rust::handlers;
use serde_json::json;

mod common;
use common::create_test_state;

#[tokio::test]
async fn test_list_growth_logs_empty() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/growth-logs", axum::routing::get(handlers::growth_logs::list_growth_logs))
        .with_state(state.clone());

    let request = Request::builder()
        .uri("/api/growth-logs")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["logs"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_create_and_get_growth_log() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/growth-logs", axum::routing::get(handlers::growth_logs::list_growth_logs))
        .route("/api/growth-logs", axum::routing::post(handlers::growth_logs::create_growth_log))
        .route("/api/growth-logs/:batch/:id", axum::routing::get(handlers::growth_logs::get_growth_log))
        .with_state(state.clone());

    // First, need to create a plant archive entry for the plant_short_name
    // Insert directly into database since we don't have plant archive API in this test
    sqlx::query("INSERT INTO plant_archive (short_name, full_name) VALUES (?, ?)")
        .bind("测试辣椒")
        .bind("测试辣椒品种")
        .execute(&state.db_pool)
        .await
        .unwrap();

    // Create a growth log
    let create_json = json!({
        "plant_short_name": "测试辣椒",
        "event_date": "2024.03.15",
        "event_type": "出芽",
        "quantity_location": "6号位",
        "details": "测试出芽记录"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/growth-logs")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    let id = json["log"]["id"].as_i64().unwrap();

    // Get the created growth log
    let get_request = Request::builder()
        .uri(format!("/api/growth-logs/batch1/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["log"]["plant_short_name"], "测试辣椒");
    assert_eq!(json["log"]["event_type"], "出芽");
}

#[tokio::test]
async fn test_update_growth_log() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/growth-logs", axum::routing::post(handlers::growth_logs::create_growth_log))
        .route("/api/growth-logs/:batch/:id", axum::routing::put(handlers::growth_logs::update_growth_log))
        .route("/api/growth-logs/:batch/:id", axum::routing::get(handlers::growth_logs::get_growth_log))
        .with_state(state.clone());

    // Insert plant archive
    sqlx::query("INSERT INTO plant_archive (short_name, full_name) VALUES (?, ?)")
        .bind("更新测试植物")
        .bind("更新测试植物品种")
        .execute(&state.db_pool)
        .await
        .unwrap();

    // Create first
    let create_json = json!({
        "plant_short_name": "更新测试植物",
        "event_date": "2024.03.10",
        "event_type": "播种",
        "details": "原始详情"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/growth-logs")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["log"]["id"].as_i64().unwrap();

    // Update
    let update_json = json!({
        "quantity_location": "3号位",
        "details": "更新后的详情"
    });

    let update_request = Request::builder()
        .method("PUT")
        .uri(format!("/api/growth-logs/batch1/{}", id))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&update_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(update_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify update
    let get_request = Request::builder()
        .uri(format!("/api/growth-logs/batch1/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["log"]["quantity_location"], "3号位");
    assert_eq!(json["log"]["details"], "更新后的详情");
}

#[tokio::test]
async fn test_delete_growth_log() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/growth-logs", axum::routing::post(handlers::growth_logs::create_growth_log))
        .route("/api/growth-logs/:batch/:id", axum::routing::delete(handlers::growth_logs::delete_growth_log))
        .route("/api/growth-logs", axum::routing::get(handlers::growth_logs::list_growth_logs))
        .with_state(state.clone());

    // Insert plant archive
    sqlx::query("INSERT INTO plant_archive (short_name, full_name) VALUES (?, ?)")
        .bind("删除测试植物")
        .bind("删除测试植物品种")
        .execute(&state.db_pool)
        .await
        .unwrap();

    // Create
    let create_json = json!({
        "plant_short_name": "删除测试植物",
        "event_date": "2024.03.10",
        "event_type": "观察",
        "details": "待删除记录"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/growth-logs")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["log"]["id"].as_i64().unwrap();

    // Delete
    let delete_request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/growth-logs/batch1/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(delete_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify deleted
    let list_request = Request::builder()
        .uri("/api/growth-logs")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(list_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let logs = json["logs"].as_array().unwrap();
    assert_eq!(logs.len(), 0);
}

#[tokio::test]
async fn test_growth_log_validation_plant_not_exists() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/growth-logs", axum::routing::post(handlers::growth_logs::create_growth_log))
        .with_state(state.clone());

    // Try to create growth log with non-existent plant short name
    let create_json = json!({
        "plant_short_name": "不存在的植物",
        "event_date": "2024.03.15",
        "event_type": "出芽"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/growth-logs")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.oneshot(create_request).await.unwrap();
    // Should return BAD_REQUEST because plant doesn't exist
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], false);
    assert!(json["message"].as_str().unwrap().contains("不存在于品种档案中"));
}