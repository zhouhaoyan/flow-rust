use serde::{Deserialize, Deserializer, Serialize, Serializer};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::FromRow;

// 自定义日期序列化/反序列化函数，支持 YYYY.MM.DD 格式
pub fn deserialize_naive_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    NaiveDate::parse_from_str(&s, "%Y.%m.%d")
        .map_err(serde::de::Error::custom)
}

pub fn serialize_naive_date<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = date.format("%Y.%m.%d").to_string();
    serializer.serialize_str(&s)
}

pub fn deserialize_optional_naive_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt {
        Some(s) => NaiveDate::parse_from_str(&s, "%Y.%m.%d")
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

pub fn serialize_optional_naive_date<S>(date: &Option<NaiveDate>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match date {
        Some(d) => {
            let s = d.format("%Y.%m.%d").to_string();
            serializer.serialize_str(&s)
        }
        None => serializer.serialize_none(),
    }
}

// 1. 品种档案 (Plant Archive)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlantArchive {
    pub id: i64,
    pub short_name: String,          // 简称
    pub full_name: Option<String>,   // 品种名称
    pub category: Option<String>,    // 种类
    pub variety_type: Option<String>, // 品种类型
    pub height_habit: Option<String>, // 株高/习性
    pub fruit_features: Option<String>, // 果实特征
    pub taste_usage: Option<String>, // 口感/用途
    pub estimated_yield: Option<String>, // 单株预估产量
    pub notes: Option<String>,       // 备注
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// 创建新品种时的输入结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePlantArchive {
    pub short_name: String,
    pub full_name: Option<String>,
    pub category: Option<String>,
    pub variety_type: Option<String>,
    pub height_habit: Option<String>,
    pub fruit_features: Option<String>,
    pub taste_usage: Option<String>,
    pub estimated_yield: Option<String>,
    pub notes: Option<String>,
}

// 2. 生长日志 (Growth Log) - 第一批和第二批共用相同结构
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GrowthLog {
    pub id: i64,
    pub plant_short_name: String,
    #[serde(serialize_with = "serialize_naive_date")]
    pub event_date: NaiveDate,  // 使用NaiveDate存储YYYY.MM.DD格式
    pub event_type: EventType,
    pub quantity_location: Option<String>, // 数量/部位
    pub details: Option<String>, // 详情记录
    pub created_at: DateTime<Utc>,
}

// 事件类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum EventType {
    播种,  // sowing
    出芽,  // germination
    假植,  // transplanting to temporary pot
    移栽,  // transplanting to final location
    死亡,  // death
    观察,  // observation
    操作,  // operation
    处理,  // treatment
}

// 创建生长日志的输入结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGrowthLog {
    pub plant_short_name: String,
    #[serde(deserialize_with = "deserialize_naive_date")]
    pub event_date: NaiveDate,
    pub event_type: EventType,
    pub quantity_location: Option<String>,
    pub details: Option<String>,
}

// 3. 产量记录 (Yield Records)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct YieldRecord {
    pub id: i64,
    pub plant_short_name: String,
    pub harvest_date: NaiveDate,
    pub quantity: Option<f64>,  // 产量（重量或数量）
    pub unit: Option<String>,   // 单位
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateYieldRecord {
    pub plant_short_name: String,
    #[serde(deserialize_with = "deserialize_naive_date")]
    pub harvest_date: NaiveDate,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub notes: Option<String>,
}

// 4. 出芽率与活苗率统计 (Germination and Survival Statistics)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GerminationStats {
    pub id: i64,
    pub batch: BatchType,
    pub plant_short_name: String,
    pub seeds_sown: i64,        // 播种数
    pub seeds_germinated: i64,  // 已出芽
    pub seeds_pending: i64,     // 待出芽
    pub seeds_dead: i64,        // 已死亡种子数
    pub germination_rate: Option<f64>, // 出芽率（计算字段）
    pub survival_rate: Option<f64>,    // 定植前活苗率（计算字段）
    pub notes: Option<String>,
    pub calculated_at: DateTime<Utc>,
}

// 批次类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum BatchType {
    第一批,  // first batch
    第二批,  // second batch
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGerminationStats {
    pub batch: BatchType,
    pub plant_short_name: String,
    pub seeds_sown: i64,
    pub seeds_germinated: i64,
    pub seeds_pending: i64,
    pub seeds_dead: i64,
    pub notes: Option<String>,
}

// 5. 育苗以外植物记录 (Non-Seedling Plant Records)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NonSeedlingRecord {
    pub id: i64,
    pub plant_name: String,     // 植物名称
    pub record_date: NaiveDate,
    pub record_type: RecordType,
    pub details: String,        // 详情
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

// 记录类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum RecordType {
    操作,  // operation
    观察,  // observation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNonSeedlingRecord {
    pub plant_name: String,
    #[serde(deserialize_with = "deserialize_naive_date")]
    pub record_date: NaiveDate,
    pub record_type: RecordType,
    pub details: String,
    pub notes: Option<String>,
}

// 6. 肥料与基质信息表 (Fertilizer and Substrate Information)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FertilizerMaterial {
    pub id: i64,
    pub name: String,           // 名称
    pub category: Option<String>, // 类别
    pub description: Option<String>, // 描述
    pub usage_instructions: Option<String>, // 使用说明
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFertilizerMaterial {
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub usage_instructions: Option<String>,
    pub notes: Option<String>,
}

// 7. 种植容器尺寸清单 (Container Size List)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContainerSize {
    pub id: i64,
    pub container_type: String, // 容器类型
    pub dimensions: Option<String>, // 尺寸
    pub quantity: Option<i64>,  // 数量
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContainerSize {
    pub container_type: String,
    pub dimensions: Option<String>,
    pub quantity: Option<i64>,
    pub notes: Option<String>,
}

// 8. 当前待办与重要提醒 (Todo and Important Reminders)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TodoReminder {
    pub id: i64,
    pub content: String,        // 内容
    pub priority: Option<Priority>, // 优先级
    #[serde(serialize_with = "serialize_optional_naive_date", deserialize_with = "deserialize_optional_naive_date")]
    pub due_date: Option<NaiveDate>, // 截止日期
    pub completed: bool,        // 是否完成
    pub completed_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,   // 备注
    pub created_at: DateTime<Utc>,
}

// 优先级枚举
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum Priority {
    高,  // high
    中,  // medium
    低,  // low
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTodoReminder {
    pub content: String,
    pub priority: Option<Priority>,
    #[serde(default, deserialize_with = "deserialize_optional_naive_date")]
    pub due_date: Option<NaiveDate>,
    pub notes: Option<String>,
}

// 通用响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

// 错误响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub message: String,
    pub error: String,
}

// 自然语言解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPlantData {
    pub event_type: Option<String>,          // 事件类型: 播种, 出芽, 假植, 移栽, 死亡, 观察, 操作, 处理
    pub plant_short_name: Option<String>,    // 植物简称 (必须与品种档案匹配)
    pub event_date: Option<String>,          // 日期 (YYYY.MM.DD格式)
    pub quantity_location: Option<String>,   // 数量/位置 (如"8粒", "6号位", "1、2、4号杯")
    pub batch: Option<String>,               // 批次: 第一批, 第二批
    pub details: Option<String>,             // 详情描述
    pub record_type: Option<String>,         // 记录类型: 操作, 观察 (用于非育苗植物)
    pub plant_name: Option<String>,          // 植物名称 (用于非育苗植物记录)
    pub is_germination_report: bool,         // 是否为出芽报告 (需要对比历史数据)
    pub is_death_report: bool,               // 是否为死亡报告
    pub is_cumulative_quantity: bool,        // 数量是否为累计总数 (需要计算新增量)
    pub raw_text: String,                    // 原始输入文本
    pub confidence: f32,                     // 解析置信度 (0.0-1.0)
    pub parsing_errors: Vec<String>,         // 解析过程中遇到的错误或警告
}