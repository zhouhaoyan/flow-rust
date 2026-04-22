use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tracing_subscriber;
use flower_rust;

/// 创建测试数据库连接池（内存数据库）
pub async fn create_test_pool() -> SqlitePool {
    // 初始化日志（仅用于测试）
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .try_init();

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database pool");

    // 运行迁移
    run_migrations(&pool).await.expect("Failed to run migrations");

    pool
}

/// 运行数据库迁移
async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    use sqlx::migrate::MigrateDatabase;

    // 确保数据库存在（内存数据库总是存在）
    if !sqlx::Sqlite::database_exists("sqlite::memory:").await? {
        sqlx::Sqlite::create_database("sqlite::memory:").await?;
    }

    // 使用 sqlx::migrate! 宏来运行迁移
    let migrator = sqlx::migrate!("./migrations"); // 相对于 crate 根目录
    migrator.run(pool).await?;

    Ok(())
}

/// 创建测试应用状态
pub async fn create_test_state() -> std::sync::Arc<flower_rust::handlers::AppState> {
    let db_pool = create_test_pool().await;
    let deepseek_client = flower_rust::deepseek::DeepSeekClient::new();
    let store = std::sync::Arc::new(flower_rust::store::Store::new());

    std::sync::Arc::new(flower_rust::handlers::AppState {
        deepseek_client,
        store,
        db_pool,
    })
}