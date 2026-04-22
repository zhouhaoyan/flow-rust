use chrono::{NaiveDate, Utc};
use regex::Regex;
use sqlx::SqlitePool;
use tracing::error;

// 验证错误类型
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub severity: ValidationSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationSeverity {
    Error,   // 必须修复的错误
    Warning, // 警告，可以继续但可能需要用户确认
    Info,    // 信息性提示
}

// 验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationError>,
    pub infos: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            infos: Vec::new(),
        }
    }

    pub fn add_error(&mut self, field: &str, message: &str) {
        self.is_valid = false;
        self.errors.push(ValidationError {
            field: field.to_string(),
            message: message.to_string(),
            severity: ValidationSeverity::Error,
        });
    }

    pub fn add_warning(&mut self, field: &str, message: &str) {
        self.warnings.push(ValidationError {
            field: field.to_string(),
            message: message.to_string(),
            severity: ValidationSeverity::Warning,
        });
    }

    pub fn add_info(&mut self, field: &str, message: &str) {
        self.infos.push(ValidationError {
            field: field.to_string(),
            message: message.to_string(),
            severity: ValidationSeverity::Info,
        });
    }

    pub fn merge(&mut self, other: ValidationResult) {
        self.is_valid = self.is_valid && other.is_valid;
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.infos.extend(other.infos);
    }
}

// 日期验证
pub fn validate_date_format(date_str: &str) -> ValidationResult {
    let mut result = ValidationResult::new();
    let date_regex = Regex::new(r"^\d{4}\.\d{2}\.\d{2}$").unwrap();

    if !date_regex.is_match(date_str) {
        result.add_error("date", "日期格式必须为 YYYY.MM.DD");
        return result;
    }

    // 尝试解析日期
    match NaiveDate::parse_from_str(date_str, "%Y.%m.%d") {
        Ok(date) => {
            // 检查日期是否在合理范围内（不超过未来1年，不早于2020年）
            let today = Utc::now().naive_utc().date();
            let min_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();

            if date > today && (date - today).num_days() > 365 {
                result.add_warning("date", "日期在未来一年之后，请确认");
            }

            if date < min_date {
                result.add_warning("date", "日期早于2020年，请确认");
            }
        }
        Err(e) => {
            result.add_error("date", &format!("无效的日期: {}", e));
        }
    }

    result
}

// 植物简称验证（检查是否存在于品种档案中）
pub async fn validate_plant_short_name(
    pool: &SqlitePool,
    short_name: &str,
) -> ValidationResult {
    let mut result = ValidationResult::new();

    if short_name.trim().is_empty() {
        result.add_error("plant_short_name", "植物简称不能为空");
        return result;
    }

    // 检查简称是否存在于品种档案中
    let exists: Result<Option<(i64,)>, _> = sqlx::query_as(
        "SELECT id FROM plant_archive WHERE short_name = ?"
    )
    .bind(short_name)
    .fetch_optional(pool)
    .await;

    match exists {
        Ok(Some(_)) => {
            // 简称存在，验证通过
            result.add_info("plant_short_name", &format!("植物简称 '{}' 存在于品种档案中", short_name));
        }
        Ok(None) => {
            result.add_error("plant_short_name", &format!("植物简称 '{}' 不存在于品种档案中", short_name));
        }
        Err(e) => {
            error!("数据库查询失败: {}", e);
            result.add_error("plant_short_name", "数据库查询失败，无法验证植物简称");
        }
    }

    result
}

// 事件类型验证
pub fn validate_event_type(event_type: &str) -> ValidationResult {
    let mut result = ValidationResult::new();
    let valid_types = vec![
        "播种", "出芽", "假植", "移栽", "死亡", "观察", "操作", "处理"
    ];

    if !valid_types.contains(&event_type) {
        result.add_error(
            "event_type",
            &format!("事件类型 '{}' 无效，必须是: {}", event_type, valid_types.join(", "))
        );
    }

    result
}

// 记录类型验证（用于非育苗植物）
pub fn validate_record_type(record_type: &str) -> ValidationResult {
    let mut result = ValidationResult::new();

    if record_type != "操作" && record_type != "观察" {
        result.add_error(
            "record_type",
            &format!("记录类型 '{}' 无效，必须是: 操作, 观察", record_type)
        );
    }

    result
}

// 批次验证
pub fn validate_batch(batch: &str) -> ValidationResult {
    let mut result = ValidationResult::new();

    if batch != "第一批" && batch != "第二批" {
        result.add_error(
            "batch",
            &format!("批次 '{}' 无效，必须是: 第一批, 第二批", batch)
        );
    }

    result
}

// 数量/位置格式验证
pub fn validate_quantity_location(quantity_location: &str) -> ValidationResult {
    let mut result = ValidationResult::new();

    if quantity_location.trim().is_empty() {
        result.add_warning("quantity_location", "数量/位置为空，某些统计可能无法计算");
        return result;
    }

    // 检查常见格式
    let patterns = vec![
        (r"^\d+粒$", "粒数格式（如'8粒'）"),
        (r"^\d+号(?:位|杯)$", "位置格式（如'6号位'或'6号杯'）"),
        (r"^\d+(?:、\d+)*号(?:位|杯)$", "多位置格式（如'1、2、4号杯'）"),
        (r"^\d+棵$", "棵数格式（如'2棵'）"),
        (r"^\d+个$", "个数格式"),
    ];

    let mut matched = false;
    for (pattern, description) in patterns {
        let re = Regex::new(pattern).unwrap();
        if re.is_match(quantity_location) {
            result.add_info("quantity_location", &format!("数量/位置格式: {}", description));
            matched = true;
            break;
        }
    }

    if !matched {
        result.add_warning(
            "quantity_location",
            &format!("数量/位置 '{}' 格式不常见，请确认", quantity_location)
        );
    }

    result
}

// 业务规则：操作 vs 观察验证
pub fn validate_operation_vs_observation(
    event_type: &str,
    _details: &str,
    raw_text: &str,
) -> ValidationResult {
    let mut result = ValidationResult::new();

    // 检查用户是否明确表示做了某事
    let operation_indicators = vec![
        "我做了", "我种了", "我施了", "我浇了", "我翻了", "我移栽了",
        "我处理了", "我进行了", "完成了", "做了", "种了", "施了"
    ];

    // 检查用户是否在询问或计划
    let question_indicators = vec![
        "能不能", "要不要", "应该", "需要", "建议", "请教",
        "怎么办", "如何", "怎样", "吗？", "？", "?"
    ];

    let is_operation = operation_indicators.iter().any(|indicator| raw_text.contains(indicator));
    let is_question = question_indicators.iter().any(|indicator| raw_text.contains(indicator));

    if event_type == "操作" && !is_operation {
        result.add_warning(
            "event_type",
            "标记为'操作'但用户可能只是在询问或计划，请确认用户确实执行了操作"
        );
    }

    if (event_type == "观察" || event_type == "操作") && is_question {
        result.add_warning(
            "event_type",
            "用户可能在询问而不是报告，请确认用户确实观察到了现象或执行了操作"
        );
    }

    // 需求.md规则：严格区分"操作"与"询问"
    if is_question && !raw_text.contains("我观察到了") {
        result.add_error(
            "business_rule",
            "用户问'能不能''要不要'不代表已经做了，只有说'我做了''我种了''我观察到了'才算操作或观察"
        );
    }

    result
}

// 出芽报告验证
pub fn validate_germination_report(
    is_germination_report: bool,
    quantity_location: &str,
    _details: &str,
) -> ValidationResult {
    let mut result = ValidationResult::new();

    if is_germination_report {
        if quantity_location.is_empty() {
            result.add_warning(
                "quantity_location",
                "出芽报告缺少数量/位置信息，无法跟踪具体出芽情况"
            );
        }

        // 检查是否包含位置信息
        if !quantity_location.contains("号") {
            result.add_warning(
                "quantity_location",
                "出芽报告建议包含具体位置（如'6号位'）以便跟踪"
            );
        }

        result.add_info(
            "germination_report",
            "出芽报告需要与历史数据对比，识别新增位置"
        );
    }

    result
}

// 死亡报告验证
pub fn validate_death_report(
    is_death_report: bool,
    event_type: &str,
) -> ValidationResult {
    let mut result = ValidationResult::new();

    if is_death_report {
        if event_type != "死亡" {
            result.add_warning(
                "event_type",
                "标记为死亡报告但事件类型不是'死亡'，请确认"
            );
        }

        result.add_info(
            "death_report",
            "死亡记录将更新活苗率但不影响出芽数"
        );
    }

    result
}

// 累计数量验证
pub fn validate_cumulative_quantity(
    is_cumulative_quantity: bool,
    quantity_location: &str,
) -> ValidationResult {
    let mut result = ValidationResult::new();

    if is_cumulative_quantity {
        if quantity_location.is_empty() {
            result.add_warning(
                "quantity_location",
                "累计数量报告缺少数量信息，无法计算新增量"
            );
        }

        result.add_info(
            "cumulative_quantity",
            "累计数量需要与历史数据对比计算新增量"
        );
    }

    result
}

// 综合验证函数
pub async fn validate_parsed_data(
    pool: &SqlitePool,
    event_type: Option<&str>,
    plant_short_name: Option<&str>,
    event_date: Option<&str>,
    quantity_location: Option<&str>,
    batch: Option<&str>,
    record_type: Option<&str>,
    is_germination_report: bool,
    is_death_report: bool,
    is_cumulative_quantity: bool,
    details: Option<&str>,
    raw_text: &str,
) -> ValidationResult {
    let mut result = ValidationResult::new();

    // 验证必填字段
    if event_type.is_none() {
        result.add_error("event_type", "事件类型不能为空");
    }

    if plant_short_name.is_none() {
        result.add_error("plant_short_name", "植物简称不能为空");
    }

    if event_date.is_none() {
        result.add_error("event_date", "日期不能为空");
    }

    // 验证各个字段
    if let Some(event_type_str) = event_type {
        result.merge(validate_event_type(event_type_str));
    }

    if let Some(plant_short_name_str) = plant_short_name {
        result.merge(validate_plant_short_name(pool, plant_short_name_str).await);
    }

    if let Some(event_date_str) = event_date {
        result.merge(validate_date_format(event_date_str));
    }

    if let Some(quantity_location_str) = quantity_location {
        result.merge(validate_quantity_location(quantity_location_str));
    }

    if let Some(batch_str) = batch {
        result.merge(validate_batch(batch_str));
    }

    if let Some(record_type_str) = record_type {
        result.merge(validate_record_type(record_type_str));
    }

    // 验证业务规则
    if let (Some(event_type_str), Some(details_str)) = (event_type, details) {
        result.merge(validate_operation_vs_observation(event_type_str, details_str, raw_text));
    }

    result.merge(validate_germination_report(
        is_germination_report,
        quantity_location.unwrap_or(""),
        details.unwrap_or(""),
    ));

    result.merge(validate_death_report(is_death_report, event_type.unwrap_or("")));

    result.merge(validate_cumulative_quantity(
        is_cumulative_quantity,
        quantity_location.unwrap_or(""),
    ));

    result
}