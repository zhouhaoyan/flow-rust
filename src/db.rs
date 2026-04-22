use sqlx::{sqlite::SqlitePoolOptions, SqlitePool, migrate::MigrateDatabase};
use std::path::Path;
use tracing::info;

// 数据库文件路径
const DATABASE_URL: &str = "sqlite:data/flower.db";

pub async fn init_db() -> Result<SqlitePool, sqlx::Error> {
    // 确保数据目录存在
    let _db_path = "data/flower.db";
    let data_dir = Path::new("data");

    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir)?;
        info!("Created data directory");
    }

    // 检查数据库是否存在，如果不存在则创建
    if !sqlx::Sqlite::database_exists(DATABASE_URL).await? {
        sqlx::Sqlite::create_database(DATABASE_URL).await?;
        info!("Created database at {}", DATABASE_URL);
    }

    // 创建连接池
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(DATABASE_URL)
        .await?;

    // 运行迁移
    run_migrations(&pool).await?;

    info!("Database initialized successfully");
    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    info!("Running database migrations...");

    // 使用 sqlx::migrate! 宏来运行迁移
    let migrator = sqlx::migrate!("./migrations");
    migrator.run(pool).await?;

    info!("Database migrations completed successfully");
    Ok(())
}

// 测试数据库连接的函数
pub async fn test_connection(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

// 获取数据库统计信息
pub async fn get_db_stats(pool: &SqlitePool) -> Result<DbStats, sqlx::Error> {
    let tables = vec![
        "plant_archive",
        "growth_log_batch1",
        "growth_log_batch2",
        "yield_records",
        "germination_stats",
        "non_seedling_records",
        "fertilizer_materials",
        "container_sizes",
        "todo_reminders",
    ];

    let mut stats = DbStats {
        total_tables: tables.len() as i64,
        table_counts: Vec::new(),
    };

    for table in tables {
        let count: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", table))
            .fetch_one(pool)
            .await
            .unwrap_or((0,));

        stats.table_counts.push(TableCount {
            table_name: table.to_string(),
            count: count.0,
        });
    }

    Ok(stats)
}

#[derive(Debug)]
pub struct DbStats {
    pub total_tables: i64,
    pub table_counts: Vec<TableCount>,
}

#[derive(Debug)]
pub struct TableCount {
    pub table_name: String,
    pub count: i64,
}