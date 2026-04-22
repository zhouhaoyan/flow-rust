use sqlx::SqlitePool;
use crate::models::ParsedPlantData;

/// 死亡记录器
/// 处理死亡事件记录，确保不影响发芽计数
pub struct DeathRecorder {
    pool: SqlitePool,
}

impl DeathRecorder {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 验证死亡事件记录
    pub async fn validate_death_record(
        &self,
        data: &ParsedPlantData,
    ) -> Result<(Vec<String>, Vec<String>), String> {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // 检查是否是死亡报告
        if !data.is_death_report {
            return Ok((warnings, errors));
        }

        // 必须有植物简称
        let plant_short_name = match &data.plant_short_name {
            Some(name) => name,
            None => {
                errors.push("死亡报告必须指定植物简称".to_string());
                return Ok((warnings, errors));
            }
        };

        // 必须有日期
        let event_date = match &data.event_date {
            Some(date) => date,
            None => {
                errors.push("死亡报告必须指定日期".to_string());
                return Ok((warnings, errors));
            }
        };

        // 检查数量/位置信息
        if data.quantity_location.is_none() {
            warnings.push("死亡报告建议包含具体位置（如'3号位死亡'）以便准确记录".to_string());
        }

        // 验证植物是否存在
        if let Err(e) = self.validate_plant_exists(plant_short_name).await {
            errors.push(e);
        }

        // 检查是否记录过该位置的死亡事件
        if let Some(location) = &data.quantity_location {
            if let Ok(already_dead) = self.check_position_already_dead(
                plant_short_name,
                location,
                event_date,
            ).await {
                if already_dead {
                    warnings.push(format!("位置 {} 可能已经记录过死亡事件", location));
                }
            }
        }

        Ok((warnings, errors))
    }

    /// 验证植物简称是否存在
    async fn validate_plant_exists(&self, plant_short_name: &str) -> Result<(), String> {
        let query = "SELECT COUNT(*) FROM plant_archive WHERE short_name = ?";
        let count: (i64,) = match sqlx::query_as(query)
            .bind(plant_short_name)
            .fetch_one(&self.pool)
            .await
        {
            Ok(row) => row,
            Err(_) => return Err(format!("无法验证植物简称 '{}' 是否存在", plant_short_name)),
        };

        if count.0 == 0 {
            return Err(format!("植物简称 '{}' 不存在于品种档案中", plant_short_name));
        }

        Ok(())
    }

    /// 检查位置是否已经记录过死亡事件
    async fn check_position_already_dead(
        &self,
        plant_short_name: &str,
        location: &str,
        event_date: &str,
    ) -> Result<bool, String> {
        // 检查第一批
        let query_batch1 = "SELECT COUNT(*) FROM growth_log_batch1 WHERE plant_short_name = ? AND event_type = '死亡' AND quantity_location LIKE ? AND event_date < ?";
        let count1: (i64,) = match sqlx::query_as(query_batch1)
            .bind(plant_short_name)
            .bind(format!("%{}%", location))
            .bind(event_date)
            .fetch_one(&self.pool)
            .await
        {
            Ok(row) => row,
            Err(_) => (0,),
        };

        // 检查第二批
        let query_batch2 = "SELECT COUNT(*) FROM growth_log_batch2 WHERE plant_short_name = ? AND event_type = '死亡' AND quantity_location LIKE ? AND event_date < ?";
        let count2: (i64,) = match sqlx::query_as(query_batch2)
            .bind(plant_short_name)
            .bind(format!("%{}%", location))
            .bind(event_date)
            .fetch_one(&self.pool)
            .await
        {
            Ok(row) => row,
            Err(_) => (0,),
        };

        Ok(count1.0 > 0 || count2.0 > 0)
    }

    /// 记录死亡事件到数据库
    pub async fn record_death(
        &self,
        data: &ParsedPlantData,
    ) -> Result<i64, String> {
        // 确定批次
        let batch = match &data.batch {
            Some(b) => b.as_str(),
            None => {
                // 根据植物类型推断批次
                let plant_short_name = data.plant_short_name.as_ref()
                    .ok_or("死亡记录需要植物简称")?;
                if self.is_batch1_plant(plant_short_name) {
                    "第一批"
                } else {
                    "第二批"
                }
            }
        };

        // 确定插入哪个表
        let table_name = match batch {
            "第一批" => "growth_log_batch1",
            "第二批" => "growth_log_batch2",
            _ => return Err(format!("无效批次: {}", batch)),
        };

        // 构建插入语句
        let query = format!(
            "INSERT INTO {} (plant_short_name, event_date, event_type, quantity_location, details) VALUES (?, ?, ?, ?, ?)",
            table_name
        );

        let plant_short_name = data.plant_short_name.as_ref()
            .ok_or("死亡记录需要植物简称")?;
        let event_date = data.event_date.as_ref()
            .ok_or("死亡记录需要日期")?;
        let quantity_location = data.quantity_location.as_deref().unwrap_or("");
        let details = data.details.as_deref().unwrap_or("");

        let result = sqlx::query(&query)
            .bind(plant_short_name)
            .bind(event_date)
            .bind("死亡")
            .bind(quantity_location)
            .bind(details)
            .execute(&self.pool)
            .await
            .map_err(|e| format!("数据库插入失败: {}", e))?;

        Ok(result.last_insert_rowid())
    }

    /// 判断植物是否属于第一批
    fn is_batch1_plant(&self, plant_short_name: &str) -> bool {
        let batch1_plants = ["辣椒", "番茄", "茄子", "甜椒", "朝天椒"];
        batch1_plants.contains(&plant_short_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use crate::models::ParsedPlantData;

    /// 创建测试数据库连接池
    async fn create_test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        // 创建测试表
        sqlx::query(
            "CREATE TABLE plant_archive (
                id INTEGER PRIMARY KEY,
                short_name TEXT NOT NULL UNIQUE,
                full_name TEXT,
                category TEXT,
                variety_type TEXT,
                height_habit TEXT,
                fruit_features TEXT,
                taste_usage TEXT,
                estimated_yield TEXT,
                notes TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE growth_log_batch1 (
                id INTEGER PRIMARY KEY,
                plant_short_name TEXT NOT NULL,
                event_date TEXT NOT NULL,
                event_type TEXT NOT NULL,
                quantity_location TEXT,
                details TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "CREATE TABLE growth_log_batch2 (
                id INTEGER PRIMARY KEY,
                plant_short_name TEXT NOT NULL,
                event_date TEXT NOT NULL,
                event_type TEXT NOT NULL,
                quantity_location TEXT,
                details TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"
        ).execute(&pool).await.unwrap();

        // 插入测试数据
        sqlx::query(
            "INSERT INTO plant_archive (short_name, full_name) VALUES (?, ?)"
        )
        .bind("辣椒")
        .bind("辣椒品种")
        .execute(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO plant_archive (short_name, full_name) VALUES (?, ?)"
        )
        .bind("番茄")
        .bind("番茄品种")
        .execute(&pool).await.unwrap();

        pool
    }

    fn create_test_data(
        event_type: Option<String>,
        plant_short_name: Option<String>,
        event_date: Option<String>,
        quantity_location: Option<String>,
        batch: Option<String>,
        is_death_report: bool,
    ) -> ParsedPlantData {
        ParsedPlantData {
            event_type,
            plant_short_name,
            event_date,
            quantity_location,
            batch,
            details: Some("测试详情".to_string()),
            record_type: None,
            plant_name: None,
            is_germination_report: false,
            is_death_report,
            is_cumulative_quantity: false,
            raw_text: "测试文本".to_string(),
            confidence: 0.9,
            parsing_errors: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_validate_death_record_valid() {
        let pool = create_test_pool().await;
        let recorder = DeathRecorder::new(pool);

        let data = create_test_data(
            Some("死亡".to_string()),
            Some("辣椒".to_string()),
            Some("2024.03.15".to_string()),
            Some("3号位".to_string()),
            Some("第一批".to_string()),
            true,
        );

        let (warnings, errors) = recorder.validate_death_record(&data).await.unwrap();
        assert!(errors.is_empty());
        // 应该没有警告，因为数据完整
        assert!(warnings.is_empty());
    }

    #[tokio::test]
    async fn test_validate_death_record_missing_plant_name() {
        let pool = create_test_pool().await;
        let recorder = DeathRecorder::new(pool);

        let data = create_test_data(
            Some("死亡".to_string()),
            None, // 缺少植物简称
            Some("2024.03.15".to_string()),
            Some("3号位".to_string()),
            Some("第一批".to_string()),
            true,
        );

        let (_warnings, errors) = recorder.validate_death_record(&data).await.unwrap();
        assert!(errors.iter().any(|e| e.contains("必须指定植物简称")));
    }

    #[tokio::test]
    async fn test_validate_death_record_missing_date() {
        let pool = create_test_pool().await;
        let recorder = DeathRecorder::new(pool);

        let data = create_test_data(
            Some("死亡".to_string()),
            Some("辣椒".to_string()),
            None, // 缺少日期
            Some("3号位".to_string()),
            Some("第一批".to_string()),
            true,
        );

        let (_warnings, errors) = recorder.validate_death_record(&data).await.unwrap();
        assert!(errors.iter().any(|e| e.contains("必须指定日期")));
    }

    #[tokio::test]
    async fn test_validate_death_record_plant_not_exists() {
        let pool = create_test_pool().await;
        let recorder = DeathRecorder::new(pool);

        let data = create_test_data(
            Some("死亡".to_string()),
            Some("不存在的植物".to_string()), // 不存在的植物
            Some("2024.03.15".to_string()),
            Some("3号位".to_string()),
            Some("第一批".to_string()),
            true,
        );

        let (_warnings, errors) = recorder.validate_death_record(&data).await.unwrap();
        assert!(errors.iter().any(|e| e.contains("不存在于品种档案中")));
    }

    #[tokio::test]
    async fn test_validate_death_record_not_death_report() {
        let pool = create_test_pool().await;
        let recorder = DeathRecorder::new(pool);

        let data = create_test_data(
            Some("观察".to_string()),
            Some("辣椒".to_string()),
            Some("2024.03.15".to_string()),
            Some("3号位".to_string()),
            Some("第一批".to_string()),
            false, // 不是死亡报告
        );

        let (warnings, errors) = recorder.validate_death_record(&data).await.unwrap();
        assert!(errors.is_empty());
        assert!(warnings.is_empty());
    }

    #[tokio::test]
    async fn test_is_batch1_plant() {
        let pool = create_test_pool().await;
        let recorder = DeathRecorder::new(pool);

        assert!(recorder.is_batch1_plant("辣椒"));
        assert!(recorder.is_batch1_plant("番茄"));
        assert!(!recorder.is_batch1_plant("西瓜")); // 不属于第一批
    }
}