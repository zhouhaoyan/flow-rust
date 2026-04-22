use axum::{
    routing::{get, post, put, delete},
    Router,
    response::Redirect,
};
use dotenv::dotenv;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use tracing_subscriber;

use flower_rust::{db, deepseek, store}; // Modules are defined in lib.rs
use flower_rust::handlers::{record_plant_data, confirm_record, get_records};

use flower_rust::handlers::plant_archive::{list_plant_archives, create_plant_archive, get_plant_archive, update_plant_archive, delete_plant_archive};
use flower_rust::handlers::growth_logs::{list_growth_logs, create_growth_log, get_growth_log, update_growth_log, delete_growth_log};
use flower_rust::handlers::statistics::{list_germination_stats, calculate_germination_stats, get_plant_germination_stats};
use flower_rust::handlers::yield_records::{list_yield_records, create_yield_record, get_yield_record, update_yield_record, delete_yield_record};
use flower_rust::handlers::non_seedling_records::{list_non_seedling_records, create_non_seedling_record, get_non_seedling_record};
use flower_rust::handlers::fertilizer_materials::{list_fertilizer_materials, create_fertilizer_material, get_fertilizer_material, update_fertilizer_material, delete_fertilizer_material};
use flower_rust::handlers::container_sizes::{list_container_sizes, create_container_size, get_container_size, update_container_size, delete_container_size};
use flower_rust::handlers::todo_reminders::{list_todo_reminders, create_todo_reminder, get_todo_reminder, update_todo_status, update_todo_reminder, delete_todo_reminder};

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize database
    let db_pool = db::init_db().await
        .expect("Failed to initialize database");

    // Initialize DeepSeek client
    let deepseek_client = deepseek::DeepSeekClient::new();

    // Initialize store (in-memory)
    let store = Arc::new(store::Store::new());

    // Create shared state
    let state = Arc::new(flower_rust::handlers::AppState {
        deepseek_client,
        store: store.clone(),
        db_pool,
    });

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port)
        .parse::<SocketAddr>()
        .expect("Invalid port");

    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/login.html") }))
        .route("/health", get(|| async { "OK" }))
        // Natural Language Processing (existing)
        .route("/api/record", post(record_plant_data))
        .route("/api/record/:id/confirm", post(confirm_record))
        .route("/api/records", get(get_records))
        // Plant Archive Management
        .route("/api/plant-archive", get(list_plant_archives))
        .route("/api/plant-archive", post(create_plant_archive))
        .route("/api/plant-archive/:id", get(get_plant_archive))
        .route("/api/plant-archive/:id", put(update_plant_archive))
        .route("/api/plant-archive/:id", delete(delete_plant_archive))
        // Growth Logs
        .route("/api/growth-logs", get(list_growth_logs))
        .route("/api/growth-logs", post(create_growth_log))
        .route("/api/growth-logs/:batch/:id", get(get_growth_log))
        .route("/api/growth-logs/:batch/:id", put(update_growth_log))
        .route("/api/growth-logs/:batch/:id", delete(delete_growth_log))
        // Germination Statistics
        .route("/api/germination-stats", get(list_germination_stats))
        .route("/api/germination-stats/calculate", get(calculate_germination_stats))
        .route("/api/germination-stats/:batch/:plant_short_name", get(get_plant_germination_stats))
        // Yield Records
        .route("/api/yield-records", get(list_yield_records))
        .route("/api/yield-records", post(create_yield_record))
        .route("/api/yield-records/:id", get(get_yield_record))
        .route("/api/yield-records/:id", put(update_yield_record))
        .route("/api/yield-records/:id", delete(delete_yield_record))
        // Non-Seedling Plant Records
        .route("/api/non-seedling-records", get(list_non_seedling_records))
        .route("/api/non-seedling-records", post(create_non_seedling_record))
        .route("/api/non-seedling-records/:id", get(get_non_seedling_record))
        // Fertilizer/Substrate Information
        .route("/api/fertilizers", get(list_fertilizer_materials))
        .route("/api/fertilizers", post(create_fertilizer_material))
        .route("/api/fertilizers/:id", get(get_fertilizer_material))
        .route("/api/fertilizers/:id", put(update_fertilizer_material))
        .route("/api/fertilizers/:id", delete(delete_fertilizer_material))
        // Container Sizes
        .route("/api/containers", get(list_container_sizes))
        .route("/api/containers", post(create_container_size))
        .route("/api/containers/:id", get(get_container_size))
        .route("/api/containers/:id", put(update_container_size))
        .route("/api/containers/:id", delete(delete_container_size))
        // Todo Reminders
        .route("/api/todo-reminders", get(list_todo_reminders))
        .route("/api/todo-reminders", post(create_todo_reminder))
        .route("/api/todo-reminders/:id", get(get_todo_reminder))
        .route("/api/todo-reminders/:id", put(update_todo_reminder))
        .route("/api/todo-reminders/:id", delete(delete_todo_reminder))
        .route("/api/todo-reminders/:id/status", post(update_todo_status))
        .nest_service("/static", ServeDir::new("static"))
        .fallback_service(ServeDir::new("static"))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("Server listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
