use sqlx::SqlitePool;
use crate::models::ParsedPlantData;

/// 发芽位置跟踪器
/// 比较每日发芽报告与历史数据，识别新增位置
pub struct GerminationTracker {
    pool: SqlitePool,
}

impl GerminationTracker {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 分析发芽报告，识别新增位置
    /// 返回: (新增位置数量, 历史位置数量, 位置列表)
    pub async fn analyze_germination_report(
        &self,
        plant_short_name: &str,
        batch: Option<&str>,
        quantity_location: Option<&str>,
        event_date: &str,
    ) -> Result<(usize, usize, Vec<String>), String> {
        // 如果没有数量/位置信息，无法跟踪
        let quantity_location = match quantity_location {
            Some(q) => q,
            None => return Ok((0, 0, Vec::new())),
        };

        // 解析位置信息（如 "6号位", "1、2、4号杯"）
        let positions = Self::parse_positions(quantity_location);
        if positions.is_empty() {
            return Ok((0, 0, Vec::new()));
        }

        // 确定批次（第一批或第二批）
        let batch = match batch {
            Some(b) => b,
            None => {
                // 根据植物类型推断批次
                if Self::is_batch1_plant(plant_short_name) {
                    "第一批"
                } else {
                    "第二批"
                }
            }
        };

        // 查询历史发芽位置
        let history_positions = self.get_historical_positions(
            plant_short_name,
            batch,
            event_date,
        ).await?;

        // 识别新增位置
        let new_positions: Vec<String> = positions
            .iter()
            .filter(|pos| !history_positions.contains(pos))
            .cloned()
            .collect();

        Ok((new_positions.len(), history_positions.len(), new_positions))
    }

    /// 解析位置字符串，提取位置标识符
    fn parse_positions(quantity_location: &str) -> Vec<String> {
        let mut positions = Vec::new();

        // 首先尝试匹配模式：数字 + "号位" 或 "号杯"
        let re = regex::Regex::new(r"(\d+)[号位杯]").unwrap();
        for cap in re.captures_iter(quantity_location) {
            if let Some(num) = cap.get(1) {
                positions.push(num.as_str().to_string());
            }
        }

        // 如果字符串包含顿号分隔，也需要单独提取数字
        // 例如 "1、2、4号杯" -> 应该提取 1, 2, 4
        if quantity_location.contains('、') {
            // 分割字符串，但保留带"号"的部分
            let parts: Vec<&str> = quantity_location.split('、').collect();
            for part in parts {
                // 如果部分包含"号"，用正则提取数字
                if part.contains("号") {
                    let re = regex::Regex::new(r"(\d+)[号位杯]").unwrap();
                    if let Some(cap) = re.captures(part) {
                        if let Some(num) = cap.get(1) {
                            positions.push(num.as_str().to_string());
                        }
                    }
                } else {
                    // 纯数字部分，提取数字
                    if let Some(num) = part.chars().take_while(|c| c.is_numeric()).collect::<String>().parse::<u32>().ok() {
                        if num > 0 {
                            positions.push(num.to_string());
                        }
                    }
                }
            }
        }

        // 去重并排序
        positions.sort();
        positions.dedup();

        positions
    }

    /// 判断植物是否属于第一批（辣椒、番茄等）
    fn is_batch1_plant(plant_short_name: &str) -> bool {
        let batch1_plants = ["辣椒", "番茄", "茄子", "甜椒", "朝天椒"];
        batch1_plants.contains(&plant_short_name)
    }

    /// 获取历史发芽位置
    async fn get_historical_positions(
        &self,
        plant_short_name: &str,
        batch: &str,
        event_date: &str,
    ) -> Result<Vec<String>, String> {
        // 确定查询哪个表
        let table_name = match batch {
            "第一批" => "growth_log_batch1",
            "第二批" => "growth_log_batch2",
            _ => return Ok(Vec::new()),
        };

        // 查询该植物在给定日期之前的所有发芽事件
        let query = format!(
            "SELECT quantity_location FROM {} WHERE plant_short_name = ? AND event_type = '出芽' AND event_date < ?",
            table_name
        );

        let rows: Vec<(String,)> = match sqlx::query_as(&query)
            .bind(plant_short_name)
            .bind(event_date)
            .fetch_all(&self.pool)
            .await
        {
            Ok(rows) => rows,
            Err(_e) => {
                // 表可能不存在或没有数据
                return Ok(Vec::new());
            }
        };

        // 从所有历史记录中提取位置
        let mut all_positions = Vec::new();
        for (location,) in rows {
            let positions = Self::parse_positions(&location);
            all_positions.extend(positions);
        }

        // 去重
        all_positions.sort();
        all_positions.dedup();

        Ok(all_positions)
    }

    /// 验证发芽报告是否符合业务规则
    pub async fn validate_germination_report(
        &self,
        data: &ParsedPlantData,
    ) -> Result<(Vec<String>, Vec<String>), String> {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // 检查是否是发芽报告
        if !data.is_germination_report {
            return Ok((warnings, errors));
        }

        // 必须有植物简称
        let plant_short_name = match &data.plant_short_name {
            Some(name) => name,
            None => {
                errors.push("发芽报告必须指定植物简称".to_string());
                return Ok((warnings, errors));
            }
        };

        // 必须有日期
        let event_date = match &data.event_date {
            Some(date) => date,
            None => {
                errors.push("发芽报告必须指定日期".to_string());
                return Ok((warnings, errors));
            }
        };

        // 分析发芽报告
        match self.analyze_germination_report(
            plant_short_name,
            data.batch.as_deref(),
            data.quantity_location.as_deref(),
            event_date,
        ).await {
            Ok((new_count, history_count, new_positions)) => {
                if new_count == 0 && history_count > 0 {
                    warnings.push(format!("发芽报告没有新增位置。历史位置数: {}", history_count));
                } else if new_count > 0 {
                    warnings.push(format!("识别到{}个新增位置: {:?}。历史位置数: {}", new_count, new_positions, history_count));
                }

                // 检查位置格式
                if let Some(location) = &data.quantity_location {
                    if !location.contains("号") && !location.contains("粒") {
                        warnings.push("发芽报告建议包含具体位置（如'6号位'）以便跟踪".to_string());
                    }
                }
            }
            Err(e) => {
                warnings.push(format!("发芽位置分析失败: {}", e));
            }
        }

        Ok((warnings, errors))
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

        // 插入测试数据：历史发芽记录
        sqlx::query(
            "INSERT INTO growth_log_batch1 (plant_short_name, event_date, event_type, quantity_location) VALUES (?, ?, ?, ?)"
        )
        .bind("辣椒")
        .bind("2024.03.10") // 早于测试日期
        .bind("出芽")
        .bind("1号位、2号位")
        .execute(&pool).await.unwrap();

        pool
    }

    fn create_test_data(
        plant_short_name: Option<String>,
        batch: Option<String>,
        quantity_location: Option<String>,
        event_date: Option<String>,
        is_germination_report: bool,
    ) -> ParsedPlantData {
        ParsedPlantData {
            event_type: Some("出芽".to_string()),
            plant_short_name,
            event_date,
            quantity_location,
            batch,
            details: Some("测试详情".to_string()),
            record_type: None,
            plant_name: None,
            is_germination_report,
            is_death_report: false,
            is_cumulative_quantity: false,
            raw_text: "测试文本".to_string(),
            confidence: 0.9,
            parsing_errors: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_analyze_germination_report_new_positions() {
        let pool = create_test_pool().await;
        let tracker = GerminationTracker::new(pool);

        // 历史已有位置1,2，新报告位置3,4
        let (new_count, history_count, new_positions) = tracker.analyze_germination_report(
            "辣椒",
            Some("第一批"),
            Some("3号位、4号位"),
            "2024.03.15",
        ).await.unwrap();

        assert_eq!(new_count, 2); // 位置3和4是新的
        assert_eq!(history_count, 2); // 历史有位置1和2
        assert_eq!(new_positions, vec!["3".to_string(), "4".to_string()]);
    }

    #[tokio::test]
    async fn test_analyze_germination_report_existing_positions() {
        let pool = create_test_pool().await;
        let tracker = GerminationTracker::new(pool);

        // 历史已有位置1,2，新报告位置1,2（重复）
        let (new_count, history_count, new_positions) = tracker.analyze_germination_report(
            "辣椒",
            Some("第一批"),
            Some("1号位、2号位"),
            "2024.03.15",
        ).await.unwrap();

        assert_eq!(new_count, 0); // 没有新位置
        assert_eq!(history_count, 2); // 历史有位置1和2
        assert!(new_positions.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_germination_report_no_location() {
        let pool = create_test_pool().await;
        let tracker = GerminationTracker::new(pool);

        // 没有位置信息
        let (new_count, history_count, new_positions) = tracker.analyze_germination_report(
            "辣椒",
            Some("第一批"),
            None,
            "2024.03.15",
        ).await.unwrap();

        assert_eq!(new_count, 0);
        assert_eq!(history_count, 0);
        assert!(new_positions.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_germination_report_batch_inference() {
        let pool = create_test_pool().await;
        let tracker = GerminationTracker::new(pool);

        // 不指定批次，根据植物类型推断
        let (new_count, history_count, _) = tracker.analyze_germination_report(
            "辣椒", // 第一批植物
            None, // 不指定批次
            Some("3号位"),
            "2024.03.15",
        ).await.unwrap();

        // 应该能正确查询第一批的表
        assert_eq!(history_count, 2); // 历史有位置1和2
        assert_eq!(new_count, 1); // 位置3是新的
    }

    #[tokio::test]
    async fn test_parse_positions() {
        // 测试各种位置字符串解析
        assert_eq!(GerminationTracker::parse_positions("6号位"), vec!["6".to_string()]);
        assert_eq!(GerminationTracker::parse_positions("1、2、4号杯"), vec!["1".to_string(), "2".to_string(), "4".to_string()]);
        assert_eq!(GerminationTracker::parse_positions("1号位和2号位"), vec!["1".to_string(), "2".to_string()]);
        assert_eq!(GerminationTracker::parse_positions("没有位置"), Vec::<String>::new());
    }

    #[tokio::test]
    async fn test_validate_germination_report_valid() {
        let pool = create_test_pool().await;
        let tracker = GerminationTracker::new(pool);

        let data = create_test_data(
            Some("辣椒".to_string()),
            Some("第一批".to_string()),
            Some("3号位".to_string()),
            Some("2024.03.15".to_string()),
            true,
        );

        let (warnings, errors) = tracker.validate_germination_report(&data).await.unwrap();
        assert!(errors.is_empty());
        // 应该有警告，提示新增位置
        assert!(warnings.iter().any(|w| w.contains("新增位置")));
    }

    #[tokio::test]
    async fn test_validate_germination_report_missing_plant_name() {
        let pool = create_test_pool().await;
        let tracker = GerminationTracker::new(pool);

        let data = create_test_data(
            None, // 缺少植物简称
            Some("第一批".to_string()),
            Some("3号位".to_string()),
            Some("2024.03.15".to_string()),
            true,
        );

        let (_warnings, errors) = tracker.validate_germination_report(&data).await.unwrap();
        assert!(errors.iter().any(|e| e.contains("必须指定植物简称")));
    }

    #[tokio::test]
    async fn test_validate_germination_report_not_germination() {
        let pool = create_test_pool().await;
        let tracker = GerminationTracker::new(pool);

        let data = create_test_data(
            Some("辣椒".to_string()),
            Some("第一批".to_string()),
            Some("3号位".to_string()),
            Some("2024.03.15".to_string()),
            false, // 不是发芽报告
        );

        let (warnings, errors) = tracker.validate_germination_report(&data).await.unwrap();
        assert!(errors.is_empty());
        assert!(warnings.is_empty());
    }
}