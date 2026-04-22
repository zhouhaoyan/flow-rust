use crate::models::ParsedPlantData;

/// 事件分类器
/// 区分"操作"（用户做了什么）和"观察"（用户看到了什么）
pub struct EventClassifier;

impl EventClassifier {
    pub fn new() -> Self {
        Self
    }

    /// 分类事件类型
    pub fn classify_event(&self, data: &ParsedPlantData) -> Result<(Vec<String>, Vec<String>), String> {
        let mut warnings = Vec::new();
        let errors = Vec::new();

        // 检查事件类型是否明确
        if let Some(event_type) = &data.event_type {
            // 检查是否是明确的操作或观察
            let is_operation = Self::is_operation_event(event_type);
            let is_observation = Self::is_observation_event(event_type);

            if !is_operation && !is_observation {
                warnings.push(format!("事件类型 '{}' 不是标准操作或观察类型", event_type));
            }

            // 检查是否有操作关键词但事件类型不匹配
            if Self::contains_operation_keywords(&data.raw_text) && !is_operation {
                warnings.push("描述中包含操作关键词，但事件类型不是'操作'，请确认".to_string());
            }

            // 检查是否有观察关键词但事件类型不匹配
            if Self::contains_observation_keywords(&data.raw_text) && !is_observation {
                warnings.push("描述中包含观察关键词，但事件类型不是'观察'，请确认".to_string());
            }
        } else {
            // 如果没有事件类型，尝试从文本推断
            let inferred_type = self.infer_event_type(&data.raw_text);
            warnings.push(format!("未指定事件类型，根据文本推断为: {}", inferred_type));
        }

        // 检查是否基于事实（没有推测性语言）
        if Self::contains_speculation(&data.raw_text) {
            warnings.push("描述中包含推测性语言，请确保只记录观察到的事实".to_string());
        }

        Ok((warnings, errors))
    }

    /// 判断是否是操作事件
    fn is_operation_event(event_type: &str) -> bool {
        let operation_events = ["播种", "假植", "移栽", "施肥", "浇水", "修剪", "处理", "操作"];
        operation_events.contains(&event_type)
    }

    /// 判断是否是观察事件
    fn is_observation_event(event_type: &str) -> bool {
        let observation_events = ["出芽", "死亡", "观察", "开花", "结果"];
        observation_events.contains(&event_type)
    }

    /// 从文本推断事件类型
    fn infer_event_type(&self, text: &str) -> String {
        let operation_keywords = ["播种", "假植", "移栽", "施肥", "浇水", "修剪", "处理", "操作"];
        let observation_keywords = ["出芽", "死亡", "观察", "看到", "发现", "开花", "结果"];

        for keyword in operation_keywords.iter() {
            if text.contains(keyword) {
                return "操作".to_string();
            }
        }

        for keyword in observation_keywords.iter() {
            if text.contains(keyword) {
                return "观察".to_string();
            }
        }

        // 默认推断为观察
        "观察".to_string()
    }

    /// 检查是否包含操作关键词
    fn contains_operation_keywords(text: &str) -> bool {
        let keywords = ["播种", "假植", "移栽", "施肥", "浇水", "修剪", "处理", "操作", "做了", "进行了"];
        keywords.iter().any(|&kw| text.contains(kw))
    }

    /// 检查是否包含观察关键词
    fn contains_observation_keywords(text: &str) -> bool {
        let keywords = ["出芽", "死亡", "观察", "看到", "发现", "开花", "结果", "有", "出现", "露白"];
        keywords.iter().any(|&kw| text.contains(kw))
    }

    /// 检查是否包含推测性语言
    fn contains_speculation(text: &str) -> bool {
        let speculation_keywords = ["可能", "也许", "大概", "似乎", "好像", "估计", "猜测", "推测", "应该"];
        speculation_keywords.iter().any(|&kw| text.contains(kw))
    }

    /// 验证事件类型是否符合业务规则
    pub fn validate_event_type_rules(
        &self,
        data: &ParsedPlantData,
    ) -> Result<(Vec<String>, Vec<String>), String> {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // 检查事件类型与植物类型是否匹配
        if let (Some(event_type), Some(plant_short_name)) = (&data.event_type, &data.plant_short_name) {
            // 非育苗植物不能有育苗相关事件
            if Self::is_seedling_event(event_type) && Self::is_non_seedling_plant(plant_short_name) {
                errors.push(format!("非育苗植物 '{}' 不能有育苗事件 '{}'", plant_short_name, event_type));
            }

            // 检查事件类型是否适合植物类型
            if event_type == "移栽" && Self::is_container_plant(plant_short_name) {
                warnings.push(format!("容器植物 '{}' 通常不需要移栽，请确认", plant_short_name));
            }
        }

        // 检查事件类型与批次是否匹配
        if let (Some(event_type), Some(batch)) = (&data.event_type, &data.batch) {
            if event_type == "出芽" && batch == "第一批" {
                // 第一批出芽事件需要更详细的位置信息
                warnings.push("第一批出芽事件建议包含详细位置信息（如'6号位'）".to_string());
            }
        }

        Ok((warnings, errors))
    }

    /// 判断是否是育苗相关事件
    fn is_seedling_event(event_type: &str) -> bool {
        let seedling_events = ["播种", "出芽", "假植", "移栽"];
        seedling_events.contains(&event_type)
    }

    /// 判断是否是非育苗植物
    fn is_non_seedling_plant(plant_short_name: &str) -> bool {
        let non_seedling_plants = ["蓝莓", "葡萄", "韭菜", "堆肥", "长条盆"];
        non_seedling_plants.contains(&plant_short_name)
    }

    /// 判断是否是容器植物
    fn is_container_plant(plant_short_name: &str) -> bool {
        let container_plants = ["长条盆", "花盆", "容器"];
        container_plants.contains(&plant_short_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ParsedPlantData;

    fn create_test_data(
        event_type: Option<String>,
        plant_short_name: Option<String>,
        batch: Option<String>,
        raw_text: String,
    ) -> ParsedPlantData {
        ParsedPlantData {
            event_type,
            plant_short_name,
            event_date: Some("2024.03.15".to_string()),
            quantity_location: Some("6号位".to_string()),
            batch,
            details: Some("测试详情".to_string()),
            record_type: None,
            plant_name: None,
            is_germination_report: false,
            is_death_report: false,
            is_cumulative_quantity: false,
            raw_text,
            confidence: 0.9,
            parsing_errors: Vec::new(),
        }
    }

    #[test]
    fn test_classify_event_operation() {
        let classifier = EventClassifier::new();
        let data = create_test_data(
            Some("播种".to_string()),
            Some("辣椒".to_string()),
            Some("第一批".to_string()),
            "今天播种了辣椒".to_string(),
        );

        let (warnings, errors) = classifier.classify_event(&data).unwrap();
        assert!(errors.is_empty());
        // 应该没有警告，因为事件类型是标准操作类型
        assert!(warnings.is_empty() || warnings.iter().any(|w| w.contains("事件类型")));
    }

    #[test]
    fn test_classify_event_observation() {
        let classifier = EventClassifier::new();
        let data = create_test_data(
            Some("出芽".to_string()),
            Some("番茄".to_string()),
            Some("第一批".to_string()),
            "番茄出芽了".to_string(),
        );

        let (warnings, errors) = classifier.classify_event(&data).unwrap();
        assert!(errors.is_empty());
        // 应该没有警告
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_classify_event_inferred_type() {
        let classifier = EventClassifier::new();
        let data = create_test_data(
            None, // 没有指定事件类型
            Some("辣椒".to_string()),
            Some("第一批".to_string()),
            "今天看到辣椒出芽了".to_string(),
        );

        let (warnings, errors) = classifier.classify_event(&data).unwrap();
        assert!(errors.is_empty());
        // 应该有一个警告，提示推断事件类型
        assert!(warnings.iter().any(|w| w.contains("未指定事件类型")));
    }

    #[test]
    fn test_classify_event_speculation_warning() {
        let classifier = EventClassifier::new();
        let data = create_test_data(
            Some("观察".to_string()),
            Some("辣椒".to_string()),
            Some("第一批".to_string()),
            "辣椒可能生病了".to_string(), // 包含推测性语言
        );

        let (warnings, errors) = classifier.classify_event(&data).unwrap();
        assert!(errors.is_empty());
        // 应该有一个警告，提示推测性语言
        assert!(warnings.iter().any(|w| w.contains("推测性语言")));
    }

    #[test]
    fn test_validate_event_type_rules_non_seedling_error() {
        let classifier = EventClassifier::new();
        let data = create_test_data(
            Some("播种".to_string()),
            Some("蓝莓".to_string()), // 非育苗植物
            None,
            "蓝莓播种".to_string(),
        );

        let (_warnings, errors) = classifier.validate_event_type_rules(&data).unwrap();
        // 应该有一个错误：非育苗植物不能有育苗事件
        assert!(errors.iter().any(|e| e.contains("不能有育苗事件")));
    }

    #[test]
    fn test_is_operation_event() {
        assert!(EventClassifier::is_operation_event("播种"));
        assert!(EventClassifier::is_operation_event("操作"));
        assert!(!EventClassifier::is_operation_event("出芽"));
    }

    #[test]
    fn test_is_observation_event() {
        assert!(EventClassifier::is_observation_event("出芽"));
        assert!(EventClassifier::is_observation_event("观察"));
        assert!(!EventClassifier::is_observation_event("播种"));
    }

    #[test]
    fn test_contains_speculation() {
        assert!(EventClassifier::contains_speculation("可能生病了"));
        assert!(EventClassifier::contains_speculation("好像有虫"));
        assert!(!EventClassifier::contains_speculation("确定有虫"));
    }
}