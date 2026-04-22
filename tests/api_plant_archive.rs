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
async fn test_list_plant_archives_empty() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/plant-archive", axum::routing::get(handlers::plant_archive::list_plant_archives))
        .with_state(state.clone());

    let request = Request::builder()
        .uri("/api/plant-archive")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["archives"].as_array().unwrap().len(), 25);
}

#[tokio::test]
async fn test_create_and_get_plant_archive() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/plant-archive", axum::routing::get(handlers::plant_archive::list_plant_archives))
        .route("/api/plant-archive", axum::routing::post(handlers::plant_archive::create_plant_archive))
        .route("/api/plant-archive/:id", axum::routing::get(handlers::plant_archive::get_plant_archive))
        .with_state(state.clone());

    // Create a plant archive
    let create_json = serde_json::json!({
        "short_name": "测试辣椒",
        "full_name": "测试辣椒品种",
        "category": "辣椒",
        "variety_type": "品种类型",
        "height_habit": "株高习性",
        "fruit_features": "果实特征",
        "taste_usage": "口感用途",
        "estimated_yield": "预估产量",
        "notes": "备注"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/plant-archive")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    let id = json["archive"]["id"].as_i64().unwrap();

    // Get the created plant archive
    let get_request = Request::builder()
        .uri(format!("/api/plant-archive/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["archive"]["short_name"], "测试辣椒");
    assert_eq!(json["archive"]["full_name"], "测试辣椒品种");
}

#[tokio::test]
async fn test_update_plant_archive() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/plant-archive", axum::routing::post(handlers::plant_archive::create_plant_archive))
        .route("/api/plant-archive/:id", axum::routing::put(handlers::plant_archive::update_plant_archive))
        .route("/api/plant-archive/:id", axum::routing::get(handlers::plant_archive::get_plant_archive))
        .with_state(state.clone());

    // Create first
    let create_json = serde_json::json!({
        "short_name": "更新测试",
        "full_name": "原始名称"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/plant-archive")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["archive"]["id"].as_i64().unwrap();

    // Update
    let update_json = serde_json::json!({
        "short_name": "更新测试",
        "full_name": "更新后的名称",
        "category": "更新类别"
    });

    let update_request = Request::builder()
        .method("PUT")
        .uri(format!("/api/plant-archive/{}", id))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&update_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(update_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify update
    let get_request = Request::builder()
        .uri(format!("/api/plant-archive/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["archive"]["full_name"], "更新后的名称");
    assert_eq!(json["archive"]["category"], "更新类别");
}

#[tokio::test]
async fn test_delete_plant_archive() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/plant-archive", axum::routing::post(handlers::plant_archive::create_plant_archive))
        .route("/api/plant-archive/:id", axum::routing::delete(handlers::plant_archive::delete_plant_archive))
        .route("/api/plant-archive", axum::routing::get(handlers::plant_archive::list_plant_archives))
        .with_state(state.clone());

    // Create
    let create_json = serde_json::json!({
        "short_name": "删除测试",
        "full_name": "待删除"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/plant-archive")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["archive"]["id"].as_i64().unwrap();

    // Delete
    let delete_request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/plant-archive/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(delete_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify deleted
    let list_request = Request::builder()
        .uri("/api/plant-archive")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(list_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let archives = json["archives"].as_array().unwrap();
    assert_eq!(archives.len(), 25);
    assert!(!archives.iter().any(|a| a["short_name"] == "删除测试"));
}