use sqlx::SqlitePool;
use crate::models::ParsedPlantData;

/// 数量计算器
/// 处理累计数量计算，将总数转换为增量
pub struct QuantityCalculator {
    pool: SqlitePool,
}

impl QuantityCalculator {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 分析数量信息，判断是否需要计算增量
    pub async fn analyze_quantity(
        &self,
        data: &ParsedPlantData,
    ) -> Result<(Vec<String>, Vec<String>), String> {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // 检查是否有累计数量标记
        if !data.is_cumulative_quantity {
            return Ok((warnings, errors));
        }

        // 必须有植物简称
        let plant_short_name = match &data.plant_short_name {
            Some(name) => name,
            None => {
                errors.push("累计数量报告必须指定植物简称".to_string());
                return Ok((warnings, errors));
            }
        };

        // 必须有日期
        let event_date = match &data.event_date {
            Some(date) => date,
            None => {
                errors.push("累计数量报告必须指定日期".to_string());
                return Ok((warnings, errors));
            }
        };

        // 解析当前数量
        let current_quantity = match Self::parse_quantity(&data.quantity_location) {
            Some(q) => q,
            None => {
                warnings.push("无法解析累计数量，请使用明确格式（如'共8粒'）".to_string());
                return Ok((warnings, errors));
            }
        };

        // 查询历史累计数量
        match self.get_historical_quantity(plant_short_name, event_date).await {
            Ok(historical_quantity) => {
                if historical_quantity > 0 {
                    let increment = current_quantity as i64 - historical_quantity as i64;
                    if increment > 0 {
                        warnings.push(format!(
                            "累计数量分析: 当前总数 {}，历史总数 {}，今日新增 {}",
                            current_quantity, historical_quantity, increment
                        ));
                    } else if increment == 0 {
                        warnings.push(format!(
                            "累计数量分析: 当前总数 {} 与历史总数相同，无新增",
                            current_quantity
                        ));
                    } else {
                        warnings.push(format!(
                            "累计数量分析: 当前总数 {} 小于历史总数 {}，可能存在数据错误",
                            current_quantity, historical_quantity
                        ));
                    }
                } else {
                    warnings.push(format!(
                        "累计数量分析: 首次报告，总数为 {}",
                        current_quantity
                    ));
                }
            }
            Err(e) => {
                warnings.push(format!("累计数量查询失败: {}", e));
            }
        }

        Ok((warnings, errors))
    }

    /// 解析数量字符串，提取数字
    fn parse_quantity(quantity_location: &Option<String>) -> Option<u32> {
        let text = quantity_location.as_ref()?;

        // 匹配数字模式
        let re = regex::Regex::new(r"(\d+)").unwrap();
        if let Some(cap) = re.captures(text) {
            if let Some(num_str) = cap.get(1) {
                return num_str.as_str().parse::<u32>().ok();
            }
        }

        // 匹配中文数字，按长度降序排列以匹配最长前缀
        let chinese_numbers = [
            ("十五", 15), ("十四", 14), ("十三", 13), ("十二", 12), ("十一", 11),
            ("十", 10), ("九", 9), ("八", 8), ("七", 7), ("六", 6),
            ("五", 5), ("四", 4), ("三", 3), ("二", 2), ("一", 1),
        ];

        for (chinese, value) in chinese_numbers.iter() {
            if text.contains(chinese) {
                return Some(*value);
            }
        }

        None
    }

    /// 获取历史累计数量
    async fn get_historical_quantity(
        &self,
        plant_short_name: &str,
        event_date: &str,
    ) -> Result<u32, String> {
        // 查询第一批
        let query_batch1 = "SELECT quantity_location FROM growth_log_batch1 WHERE plant_short_name = ? AND event_date < ? AND (event_type = '出芽' OR event_type = '播种') ORDER BY event_date DESC LIMIT 1";

        let row1: Option<(String,)> = match sqlx::query_as(query_batch1)
            .bind(plant_short_name)
            .bind(event_date)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(row) => row,
            Err(_) => None,
        };

        // 查询第二批
        let query_batch2 = "SELECT quantity_location FROM growth_log_batch2 WHERE plant_short_name = ? AND event_date < ? AND (event_type = '出芽' OR event_type = '播种') ORDER BY event_date DESC LIMIT 1";

        let row2: Option<(String,)> = match sqlx::query_as(query_batch2)
            .bind(plant_short_name)
            .bind(event_date)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(row) => row,
            Err(_) => None,
        };

        // 解析数量
        let quantities = [row1, row2].iter()
            .filter_map(|row| row.as_ref())
            .filter_map(|(text,)| Self::parse_quantity(&Some(text.clone())))
            .collect::<Vec<u32>>();

        // 取最大值作为历史累计数量
        Ok(quantities.into_iter().max().unwrap_or(0))
    }

    /// 验证数量格式
    pub fn validate_quantity_format(
        &self,
        data: &ParsedPlantData,
    ) -> Result<(Vec<String>, Vec<String>), String> {
        let mut warnings = Vec::new();
        let errors = Vec::new();

        if let Some(quantity_location) = &data.quantity_location {
            // 检查是否包含数量单位
            let has_unit = quantity_location.contains("粒") ||
                          quantity_location.contains("个") ||
                          quantity_location.contains("株") ||
                          quantity_location.contains("公斤");

            if !has_unit {
                warnings.push("数量/位置信息建议包含单位（如'粒'、'个'、'株'）".to_string());
            }

            // 检查格式是否清晰
            if quantity_location.contains("共") || quantity_location.contains("累计") || quantity_location.contains("总计") {
                warnings.push("检测到累计数量关键词（'共'、'累计'、'总计'），系统将尝试计算增量".to_string());
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

        // 插入测试数据：历史数量记录
        sqlx::query(
            "INSERT INTO growth_log_batch1 (plant_short_name, event_date, event_type, quantity_location) VALUES (?, ?, ?, ?)"
        )
        .bind("辣椒")
        .bind("2024.03.10") // 早于测试日期
        .bind("出芽")
        .bind("共5粒")
        .execute(&pool).await.unwrap();

        pool
    }

    fn create_test_data(
        plant_short_name: Option<String>,
        event_date: Option<String>,
        quantity_location: Option<String>,
        is_cumulative_quantity: bool,
    ) -> ParsedPlantData {
        ParsedPlantData {
            event_type: Some("出芽".to_string()),
            plant_short_name,
            event_date,
            quantity_location,
            batch: Some("第一批".to_string()),
            details: Some("测试详情".to_string()),
            record_type: None,
            plant_name: None,
            is_germination_report: false,
            is_death_report: false,
            is_cumulative_quantity,
            raw_text: "测试文本".to_string(),
            confidence: 0.9,
            parsing_errors: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_analyze_quantity_increment() {
        let pool = create_test_pool().await;
        let calculator = QuantityCalculator::new(pool);

        let data = create_test_data(
            Some("辣椒".to_string()),
            Some("2024.03.15".to_string()),
            Some("共8粒".to_string()), // 当前总数8，历史总数5，增量3
            true,
        );

        let (warnings, errors) = calculator.analyze_quantity(&data).await.unwrap();
        assert!(errors.is_empty());
        assert!(warnings.iter().any(|w| w.contains("今日新增 3")));
    }

    #[tokio::test]
    async fn test_analyze_quantity_no_increment() {
        let pool = create_test_pool().await;
        let calculator = QuantityCalculator::new(pool);

        let data = create_test_data(
            Some("辣椒".to_string()),
            Some("2024.03.15".to_string()),
            Some("共5粒".to_string()), // 当前总数5，历史总数5，无增量
            true,
        );

        let (warnings, errors) = calculator.analyze_quantity(&data).await.unwrap();
        assert!(errors.is_empty());
        assert!(warnings.iter().any(|w| w.contains("无新增")));
    }

    #[tokio::test]
    async fn test_analyze_quantity_first_report() {
        let pool = create_test_pool().await;
        let calculator = QuantityCalculator::new(pool);

        let data = create_test_data(
            Some("番茄".to_string()), // 没有历史记录
            Some("2024.03.15".to_string()),
            Some("共10粒".to_string()),
            true,
        );

        let (warnings, errors) = calculator.analyze_quantity(&data).await.unwrap();
        assert!(errors.is_empty());
        assert!(warnings.iter().any(|w| w.contains("首次报告")));
    }

    #[tokio::test]
    async fn test_analyze_quantity_decrease_warning() {
        let pool = create_test_pool().await;
        let calculator = QuantityCalculator::new(pool);

        let data = create_test_data(
            Some("辣椒".to_string()),
            Some("2024.03.15".to_string()),
            Some("共3粒".to_string()), // 当前总数3，历史总数5，减少2
            true,
        );

        let (warnings, errors) = calculator.analyze_quantity(&data).await.unwrap();
        assert!(errors.is_empty());
        assert!(warnings.iter().any(|w| w.contains("小于历史总数")));
    }

    #[tokio::test]
    async fn test_analyze_quantity_missing_plant_name() {
        let pool = create_test_pool().await;
        let calculator = QuantityCalculator::new(pool);

        let data = create_test_data(
            None, // 缺少植物简称
            Some("2024.03.15".to_string()),
            Some("共8粒".to_string()),
            true,
        );

        let (_warnings, errors) = calculator.analyze_quantity(&data).await.unwrap();
        assert!(errors.iter().any(|e| e.contains("必须指定植物简称")));
    }

    #[tokio::test]
    async fn test_analyze_quantity_not_cumulative() {
        let pool = create_test_pool().await;
        let calculator = QuantityCalculator::new(pool);

        let data = create_test_data(
            Some("辣椒".to_string()),
            Some("2024.03.15".to_string()),
            Some("共8粒".to_string()),
            false, // 不是累计数量
        );

        let (warnings, errors) = calculator.analyze_quantity(&data).await.unwrap();
        assert!(errors.is_empty());
        assert!(warnings.is_empty()); // 不应该有警告
    }

    #[test]
    fn test_parse_quantity() {
        // 测试解析数字
        assert_eq!(QuantityCalculator::parse_quantity(&Some("共8粒".to_string())), Some(8));
        assert_eq!(QuantityCalculator::parse_quantity(&Some("5号位".to_string())), Some(5));
        assert_eq!(QuantityCalculator::parse_quantity(&Some("十五粒".to_string())), Some(15));
        assert_eq!(QuantityCalculator::parse_quantity(&Some("没有数字".to_string())), None);
        assert_eq!(QuantityCalculator::parse_quantity(&None), None);
    }

    #[tokio::test]
    async fn test_validate_quantity_format() {
        let pool = create_test_pool().await;
        let calculator = QuantityCalculator::new(pool);

        let data = create_test_data(
            Some("辣椒".to_string()),
            Some("2024.03.15".to_string()),
            Some("8粒".to_string()),
            false,
        );

        let (warnings, errors) = calculator.validate_quantity_format(&data).unwrap();
        assert!(errors.is_empty());
        // 有单位"粒"，应该没有警告
        assert!(warnings.is_empty() || !warnings.iter().any(|w| w.contains("单位")));

        // 测试没有单位的情况
        let data2 = create_test_data(
            Some("辣椒".to_string()),
            Some("2024.03.15".to_string()),
            Some("8".to_string()), // 没有单位
            false,
        );
        let (warnings2, _) = calculator.validate_quantity_format(&data2).unwrap();
        assert!(warnings2.iter().any(|w| w.contains("单位")));
    }
}