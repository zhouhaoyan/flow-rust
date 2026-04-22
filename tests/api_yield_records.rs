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
async fn test_list_yield_records_empty() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/yield-records", axum::routing::get(handlers::yield_records::list_yield_records))
        .with_state(state.clone());

    let request = Request::builder()
        .uri("/api/yield-records")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["yields"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_create_and_get_yield_record() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/yield-records", axum::routing::get(handlers::yield_records::list_yield_records))
        .route("/api/yield-records", axum::routing::post(handlers::yield_records::create_yield_record))
        .route("/api/yield-records/:id", axum::routing::get(handlers::yield_records::get_yield_record))
        .with_state(state.clone());

    // Create a yield record
    let create_json = serde_json::json!({
        "plant_short_name": "线椒",
        "harvest_date": "2026.04.21",
        "quantity": 2.5,
        "unit": "公斤",
        "notes": "测试产量记录"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/yield-records")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    let id = json["yield_record"]["id"].as_i64().unwrap();

    // Get the created yield record
    let get_request = Request::builder()
        .uri(format!("/api/yield-records/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["yield_record"]["plant_short_name"], "线椒");
    assert_eq!(json["yield_record"]["quantity"], 2.5);
    assert_eq!(json["yield_record"]["unit"], "公斤");
}

#[tokio::test]
async fn test_update_yield_record() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/yield-records", axum::routing::post(handlers::yield_records::create_yield_record))
        .route("/api/yield-records/:id", axum::routing::put(handlers::yield_records::update_yield_record))
        .route("/api/yield-records/:id", axum::routing::get(handlers::yield_records::get_yield_record))
        .with_state(state.clone());

    // Create first
    let create_json = serde_json::json!({
        "plant_short_name": "线椒",
        "harvest_date": "2026.04.21",
        "quantity": 2.5,
        "unit": "公斤"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/yield-records")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["yield_record"]["id"].as_i64().unwrap();

    // Update
    let update_json = serde_json::json!({
        "quantity": 3.0,
        "unit": "千克",
        "notes": "更新后的记录"
    });

    let update_request = Request::builder()
        .method("PUT")
        .uri(format!("/api/yield-records/{}", id))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&update_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(update_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify update
    let get_request = Request::builder()
        .uri(format!("/api/yield-records/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["yield_record"]["quantity"], 3.0);
    assert_eq!(json["yield_record"]["unit"], "千克");
    assert_eq!(json["yield_record"]["notes"], "更新后的记录");
}

#[tokio::test]
async fn test_delete_yield_record() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/yield-records", axum::routing::post(handlers::yield_records::create_yield_record))
        .route("/api/yield-records/:id", axum::routing::delete(handlers::yield_records::delete_yield_record))
        .route("/api/yield-records", axum::routing::get(handlers::yield_records::list_yield_records))
        .with_state(state.clone());

    // Create
    let create_json = serde_json::json!({
        "plant_short_name": "线椒",
        "harvest_date": "2026.04.21",
        "quantity": 2.5,
        "unit": "公斤"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/yield-records")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(create_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = json["yield_record"]["id"].as_i64().unwrap();

    // Delete
    let delete_request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/yield-records/{}", id))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(delete_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify deleted
    let list_request = Request::builder()
        .uri("/api/yield-records")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(list_request).await.unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let yields = json["yields"].as_array().unwrap();
    assert_eq!(yields.len(), 0);
}

#[tokio::test]
async fn test_yield_record_validation_plant_not_exists() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/yield-records", axum::routing::post(handlers::yield_records::create_yield_record))
        .with_state(state.clone());

    // Try to create yield record with non-existent plant short name
    let create_json = serde_json::json!({
        "plant_short_name": "不存在的植物",
        "harvest_date": "2026.04.21",
        "quantity": 2.5,
        "unit": "公斤"
    });

    let create_request = Request::builder()
        .method("POST")
        .uri("/api/yield-records")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&create_json).unwrap()))
        .unwrap();

    let response = app.oneshot(create_request).await.unwrap();
    // Should return BAD_REQUEST (400)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], false);
    assert!(json["message"].as_str().unwrap().contains("不存在"));
}