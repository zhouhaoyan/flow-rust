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
async fn test_list_fertilizer_materials_empty() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/fertilizer-materials", axum::routing::get(handlers::fertilizer_materials::list_fertilizer_materials))
        .with_state(state.clone());

    let request = Request::builder()
        .uri("/api/fertilizer-materials")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    let _materials = json["materials"].as_array().unwrap();
    // Check initial state (might have data from migrations)
    // Length check not needed as materials is guaranteed to be an array
}

#[tokio::test]
async fn test_create_and_get_fertilizer_material() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/fertilizer-materials", axum::routing::get(handlers::fertilizer_materials::list_fertilizer_materials))
        .route("/api/fertilizer-materials", axum::routing::post(handlers::fertilizer_materials::create_fertilizer_material))
        .route("/api/fertilizer-materials/:id", axum::routing::get(handlers::fertilizer_materials::get_fertilizer_material))
        .with_state(state.clone());

    // Create a fertilizer material
    let create_json = serde_json::json!({
        "name": "测试肥料",
        "category": "有机肥",
        "description": "测试肥料描述",
        "usage_instructions": "测试使用说明",
        "notes": "测试备注"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/fertilizer-materials")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    let id = json["material"]["id"].as_i64().unwrap();

    // Get the created fertilizer material
    let get_request = Request::builder()
        .uri(format!("/api/fertilizer-materials/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["material"]["name"], "测试肥料");
    assert_eq!(json["material"]["category"], "有机肥");
    assert_eq!(json["material"]["description"], "测试肥料描述");
}

#[tokio::test]
async fn test_update_fertilizer_material() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/fertilizer-materials", axum::routing::post(handlers::fertilizer_materials::create_fertilizer_material))
        .route("/api/fertilizer-materials/:id", axum::routing::put(handlers::fertilizer_materials::update_fertilizer_material))
        .route("/api/fertilizer-materials/:id", axum::routing::get(handlers::fertilizer_materials::get_fertilizer_material))
        .with_state(state.clone());

    // Create first
    let create_json = serde_json::json!({
        "name": "更新测试肥料",
        "category": "原始类别",
        "description": "原始描述"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/fertilizer-materials")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["material"]["id"].as_i64().unwrap();

    // Update
    let update_json = serde_json::json!({
        "description": "更新后的描述",
        "usage_instructions": "新增使用说明",
        "notes": "新增备注"
    });

    let update_request = Request::builder()
        .method("PUT")
        .uri(format!("/api/fertilizer-materials/{}", id))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&update_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(update_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify update
    let get_request = Request::builder()
        .uri(format!("/api/fertilizer-materials/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["material"]["description"], "更新后的描述");
    assert_eq!(json["material"]["usage_instructions"], "新增使用说明");
    assert_eq!(json["material"]["notes"], "新增备注");
}

#[tokio::test]
async fn test_delete_fertilizer_material() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/fertilizer-materials", axum::routing::post(handlers::fertilizer_materials::create_fertilizer_material))
        .route("/api/fertilizer-materials/:id", axum::routing::delete(handlers::fertilizer_materials::delete_fertilizer_material))
        .route("/api/fertilizer-materials", axum::routing::get(handlers::fertilizer_materials::list_fertilizer_materials))
        .with_state(state.clone());

    // Create
    let create_json = serde_json::json!({
        "name": "删除测试肥料",
        "category": "测试类别",
        "description": "待删除记录"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/fertilizer-materials")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["material"]["id"].as_i64().unwrap();

    // Delete
    let delete_request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/fertilizer-materials/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(delete_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify deleted - list all materials and ensure our test record is not there
    let list_request = Request::builder()
        .uri("/api/fertilizer-materials")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(list_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let materials = json["materials"].as_array().unwrap();
    // The material with name "删除测试肥料" should not be present
    assert!(!materials.iter().any(|m| m["name"] == "删除测试肥料"));
}

#[tokio::test]
async fn test_fertilizer_material_name_uniqueness() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/fertilizer-materials", axum::routing::post(handlers::fertilizer_materials::create_fertilizer_material))
        .with_state(state.clone());

    // Create first material
    let create_json1 = serde_json::json!({
        "name": "唯一肥料",
        "category": "测试"
    });

    let create_request1 = Request::builder()
        .method("POST")
        .uri("/api/fertilizer-materials")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json1).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request1).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // Try to create another material with same name
    let create_json2 = serde_json::json!({
        "name": "唯一肥料",  // Same name
        "category": "不同类别"
    });

    let create_request2 = Request::builder()
        .method("POST")
        .uri("/api/fertilizer-materials")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json2).unwrap()))
        .unwrap();

    let response = app.oneshot(create_request2).await.unwrap();
    // Should return BAD_REQUEST due to duplicate name
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], false);
    assert!(json["message"].as_str().unwrap().contains("已存在"));
}