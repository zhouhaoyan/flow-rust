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
async fn test_list_germination_stats_empty() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/germination-stats", axum::routing::get(handlers::statistics::list_germination_stats))
        .with_state(state.clone());

    let request = Request::builder()
        .uri("/api/germination-stats")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["stats"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_create_and_get_germination_stats() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/germination-stats", axum::routing::get(handlers::statistics::list_germination_stats))
        .route("/api/germination-stats/:batch/:plant_short_name", axum::routing::get(handlers::statistics::get_plant_germination_stats))
        .with_state(state.clone());

    // Insert germination stats directly into database since there's no POST endpoint
    // Need to insert a plant archive entry first for referential integrity
    sqlx::query("INSERT INTO plant_archive (short_name, full_name) VALUES (?, ?)")
        .bind("测试辣椒")
        .bind("测试辣椒品种")
        .execute(&state.db_pool)
        .await
        .unwrap();

    // Insert germination stats directly
    sqlx::query(
        "INSERT INTO germination_stats (batch, plant_short_name, seeds_sown, seeds_germinated, seeds_pending, seeds_dead, germination_rate, survival_rate, notes) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("第一批")
    .bind("测试辣椒")
    .bind(10) // seeds_sown
    .bind(7)  // seeds_germinated
    .bind(2)  // seeds_pending
    .bind(1)  // seeds_dead
    .bind(70.0) // germination_rate
    .bind(85.0) // survival_rate
    .bind("测试统计")
    .execute(&state.db_pool)
    .await
    .unwrap();

    // Get the ID of inserted stat (optional, not needed for get endpoint which uses batch+plant_short_name)

    // List all stats
    let list_request = Request::builder()
        .uri("/api/germination-stats")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(list_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    let stats = json["stats"].as_array().unwrap();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0]["plant_short_name"], "测试辣椒");
    assert_eq!(stats[0]["batch"], "第一批");

    // Get specific stat by batch and plant_short_name
    let get_request = Request::builder()
        .uri("/api/germination-stats/第一批/测试辣椒")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["stat"]["plant_short_name"], "测试辣椒");
    assert_eq!(json["stat"]["batch"], "第一批");
    assert_eq!(json["stat"]["seeds_sown"], 10);
    assert_eq!(json["stat"]["seeds_germinated"], 7);
    assert_eq!(json["stat"]["seeds_dead"], 1);
    assert_eq!(json["stat"]["germination_rate"], 70.0);
}

#[tokio::test]
async fn test_list_germination_stats_with_filter() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/germination-stats", axum::routing::get(handlers::statistics::list_germination_stats))
        .with_state(state.clone());

    // Insert plant archive entries
    sqlx::query("INSERT INTO plant_archive (short_name, full_name) VALUES (?, ?)")
        .bind("辣椒")
        .bind("辣椒品种")
        .execute(&state.db_pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO plant_archive (short_name, full_name) VALUES (?, ?)")
        .bind("番茄")
        .bind("番茄品种")
        .execute(&state.db_pool)
        .await
        .unwrap();

    // Insert multiple germination stats
    sqlx::query(
        "INSERT INTO germination_stats (batch, plant_short_name, seeds_sown, seeds_germinated, seeds_pending, seeds_dead) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind("第一批")
    .bind("辣椒")
    .bind(10).bind(7).bind(2).bind(1)
    .execute(&state.db_pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO germination_stats (batch, plant_short_name, seeds_sown, seeds_germinated, seeds_pending, seeds_dead) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind("第一批")
    .bind("番茄")
    .bind(15).bind(12).bind(2).bind(1)
    .execute(&state.db_pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO germination_stats (batch, plant_short_name, seeds_sown, seeds_germinated, seeds_pending, seeds_dead) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind("第二批")
    .bind("番茄")
    .bind(8).bind(6).bind(1).bind(1)
    .execute(&state.db_pool)
    .await
    .unwrap();

    // Test filter by batch
    let request = Request::builder()
        .uri("/api/germination-stats?batch=第一批")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let stats = json["stats"].as_array().unwrap();
    assert_eq!(stats.len(), 2); // 辣椒 and 番茄 in batch 1

    // Test filter by plant_short_name
    let request = Request::builder()
        .uri("/api/germination-stats?plant_short_name=番茄")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let stats = json["stats"].as_array().unwrap();
    assert_eq!(stats.len(), 2); // 番茄 in both batches

    // Test filter by both batch and plant_short_name
    let request = Request::builder()
        .uri("/api/germination-stats?batch=第二批&plant_short_name=番茄")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let stats = json["stats"].as_array().unwrap();
    assert_eq!(stats.len(), 1); // 番茄 in batch 2 only
    assert_eq!(stats[0]["batch"], "第二批");
}

#[tokio::test]
async fn test_get_germination_stats_not_found() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/germination-stats/:batch/:plant_short_name", axum::routing::get(handlers::statistics::get_plant_germination_stats))
        .with_state(state.clone());

    // Try to get non-existent germination stats
    let request = Request::builder()
        .uri("/api/germination-stats/第一批/不存在的植物")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should return NOT_FOUND (404)
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], false);
    assert!(json["message"].as_str().unwrap().contains("未找到"));
}

#[tokio::test]
async fn test_calculate_germination_stats() {
    let state = create_test_state().await;
    let app = Router::new()
        .route("/api/germination-stats/calculate", axum::routing::get(handlers::statistics::calculate_germination_stats))
        .with_state(state.clone());

    // Insert some germination stats data to be returned (calculate endpoint currently just returns existing stats)
    sqlx::query("INSERT INTO plant_archive (short_name, full_name) VALUES (?, ?)")
        .bind("辣椒")
        .bind("辣椒品种")
        .execute(&state.db_pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO germination_stats (batch, plant_short_name, seeds_sown, seeds_germinated, seeds_pending, seeds_dead) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind("第一批")
    .bind("辣椒")
    .bind(10).bind(7).bind(2).bind(1)
    .execute(&state.db_pool)
    .await
    .unwrap();

    let request = Request::builder()
        .uri("/api/germination-stats/calculate")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024 * 2).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], true);
    let stats = json["stats"].as_array().unwrap();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0]["plant_short_name"], "辣椒");
}