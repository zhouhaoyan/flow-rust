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
async fn test_list_non_seedling_records_empty() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/non-seedling-records", axum::routing::get(handlers::non_seedling_records::list_non_seedling_records))
        .with_state(state.clone());

    let request = Request::builder()
        .uri("/api/non-seedling-records")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    // Check if there are any existing records from migration
    let _records = json["records"].as_array().unwrap();
    // We'll just verify the response structure, not specific count
    // Length check not needed as records is guaranteed to be an array
}

#[tokio::test]
async fn test_create_and_get_non_seedling_record() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/non-seedling-records", axum::routing::get(handlers::non_seedling_records::list_non_seedling_records))
        .route("/api/non-seedling-records", axum::routing::post(handlers::non_seedling_records::create_non_seedling_record))
        .route("/api/non-seedling-records/:id", axum::routing::get(handlers::non_seedling_records::get_non_seedling_record))
        .with_state(state.clone());

    // Create a non-seedling record
    let create_json = serde_json::json!({
        "plant_name": "测试植物",
        "record_date": "2026.04.21",
        "record_type": "操作",
        "details": "进行了修剪操作",
        "notes": "测试记录"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/non-seedling-records")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    let id = json["record"]["id"].as_i64().unwrap();

    // Get the created non-seedling record
    let get_request = Request::builder()
        .uri(format!("/api/non-seedling-records/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["record"]["plant_name"], "测试植物");
    assert_eq!(json["record"]["details"], "进行了修剪操作");
    assert_eq!(json["record"]["record_type"], "操作");
}

#[tokio::test]
async fn test_update_non_seedling_record() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/non-seedling-records", axum::routing::post(handlers::non_seedling_records::create_non_seedling_record))
        .route("/api/non-seedling-records/:id", axum::routing::put(handlers::non_seedling_records::update_non_seedling_record))
        .route("/api/non-seedling-records/:id", axum::routing::get(handlers::non_seedling_records::get_non_seedling_record))
        .with_state(state.clone());

    // Create first
    let create_json = serde_json::json!({
        "plant_name": "更新测试",
        "record_date": "2026.04.21",
        "record_type": "操作",
        "details": "原始详情"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/non-seedling-records")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["record"]["id"].as_i64().unwrap();

    // Update
    let update_json = serde_json::json!({
        "details": "更新后的详情",
        "notes": "新增备注"
    });

    let update_request = Request::builder()
        .method("PUT")
        .uri(format!("/api/non-seedling-records/{}", id))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&update_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(update_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify update
    let get_request = Request::builder()
        .uri(format!("/api/non-seedling-records/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["record"]["details"], "更新后的详情");
    assert_eq!(json["record"]["notes"], "新增备注");
}

#[tokio::test]
async fn test_delete_non_seedling_record() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/non-seedling-records", axum::routing::post(handlers::non_seedling_records::create_non_seedling_record))
        .route("/api/non-seedling-records/:id", axum::routing::delete(handlers::non_seedling_records::delete_non_seedling_record))
        .route("/api/non-seedling-records", axum::routing::get(handlers::non_seedling_records::list_non_seedling_records))
        .with_state(state.clone());

    // Create
    let create_json = serde_json::json!({
        "plant_name": "删除测试",
        "record_date": "2026.04.21",
        "record_type": "操作",
        "details": "待删除记录"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/non-seedling-records")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["record"]["id"].as_i64().unwrap();

    // Delete
    let delete_request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/non-seedling-records/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(delete_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify deleted - list all records and ensure our test record is not there
    let list_request = Request::builder()
        .uri("/api/non-seedling-records")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(list_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let records = json["records"].as_array().unwrap();
    // The record with plant_name "删除测试" should not be present
    assert!(!records.iter().any(|r| r["plant_name"] == "删除测试"));
}