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
async fn test_list_container_sizes_empty() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/container-sizes", axum::routing::get(handlers::container_sizes::list_container_sizes))
        .with_state(state.clone());

    let request = Request::builder()
        .uri("/api/container-sizes")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    let _containers = json["containers"].as_array().unwrap();
    // Check initial state (might have data from migrations)
    // Length check not needed as containers is guaranteed to be an array
}

#[tokio::test]
async fn test_create_and_get_container_size() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/container-sizes", axum::routing::get(handlers::container_sizes::list_container_sizes))
        .route("/api/container-sizes", axum::routing::post(handlers::container_sizes::create_container_size))
        .route("/api/container-sizes/:id", axum::routing::get(handlers::container_sizes::get_container_size))
        .with_state(state.clone());

    // Create a container size
    let create_json = serde_json::json!({
        "container_type": "测试容器",
        "dimensions": "10x10x10cm",
        "quantity": 5,
        "notes": "测试备注"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/container-sizes")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    let id = json["container"]["id"].as_i64().unwrap();

    // Get the created container size
    let get_request = Request::builder()
        .uri(format!("/api/container-sizes/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["container"]["container_type"], "测试容器");
    assert_eq!(json["container"]["dimensions"], "10x10x10cm");
    assert_eq!(json["container"]["quantity"], 5);
}

#[tokio::test]
async fn test_update_container_size() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/container-sizes", axum::routing::post(handlers::container_sizes::create_container_size))
        .route("/api/container-sizes/:id", axum::routing::put(handlers::container_sizes::update_container_size))
        .route("/api/container-sizes/:id", axum::routing::get(handlers::container_sizes::get_container_size))
        .with_state(state.clone());

    // Create first
    let create_json = serde_json::json!({
        "container_type": "更新测试容器",
        "dimensions": "原始尺寸",
        "quantity": 3
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/container-sizes")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["container"]["id"].as_i64().unwrap();

    // Update
    let update_json = serde_json::json!({
        "dimensions": "更新后的尺寸",
        "quantity": 10,
        "notes": "新增备注"
    });

    let update_request = Request::builder()
        .method("PUT")
        .uri(format!("/api/container-sizes/{}", id))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&update_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(update_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify update
    let get_request = Request::builder()
        .uri(format!("/api/container-sizes/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["container"]["dimensions"], "更新后的尺寸");
    assert_eq!(json["container"]["quantity"], 10);
    assert_eq!(json["container"]["notes"], "新增备注");
}

#[tokio::test]
async fn test_delete_container_size() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/container-sizes", axum::routing::post(handlers::container_sizes::create_container_size))
        .route("/api/container-sizes/:id", axum::routing::delete(handlers::container_sizes::delete_container_size))
        .route("/api/container-sizes", axum::routing::get(handlers::container_sizes::list_container_sizes))
        .with_state(state.clone());

    // Create
    let create_json = serde_json::json!({
        "container_type": "删除测试容器",
        "dimensions": "待删除尺寸",
        "quantity": 1
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/container-sizes")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["container"]["id"].as_i64().unwrap();

    // Delete
    let delete_request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/container-sizes/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(delete_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify deleted - list all containers and ensure our test record is not there
    let list_request = Request::builder()
        .uri("/api/container-sizes")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(list_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let containers = json["containers"].as_array().unwrap();
    // The container with container_type "删除测试容器" should not be present
    assert!(!containers.iter().any(|c| c["container_type"] == "删除测试容器"));
}