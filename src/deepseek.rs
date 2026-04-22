use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use chrono::Utc;
use std::env;
use crate::models::ParsedPlantData;


pub struct DeepSeekClient {
    client: Option<Client<OpenAIConfig>>,
    is_mock: bool,
}

impl DeepSeekClient {
    pub fn new() -> Self {
        let api_key = env::var("DEEPSEEK_API_KEY").unwrap_or_else(|_| "mock".to_string());
        let is_mock = api_key == "mock";

        if is_mock {
            tracing::info!("Using mock DeepSeek client");
            DeepSeekClient {
                client: None,
                is_mock,
            }
        } else {
            let base_url = env::var("DEEPSEEK_BASE_URL")
                .unwrap_or_else(|_| "https://api.deepseek.com".to_string());

            let config = OpenAIConfig::new()
                .with_api_key(api_key)
                .with_api_base(base_url);
            let client = Client::with_config(config);

            DeepSeekClient {
                client: Some(client),
                is_mock,
            }
        }
    }

    // 向后兼容的旧解析方法
    pub async fn parse_plant_text(
        &self,
        text: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if self.is_mock {
            // Mock response based on input text
            let mock_json = if text.contains("黄瓜") {
                r#"{"plant_type":"黄瓜","action":"记录","quantity":1,"notes":"有一粒露白"}"#
            } else if text.contains("土豆") {
                r#"{"plant_type":"土豆","action":"培土","quantity":2,"notes":"有两棵可以培土了"}"#
            } else {
                r#"{"plant_type":null,"action":null,"quantity":null,"notes":"未识别"}"#
            };
            Ok(mock_json.to_string())
        } else {
            // Real API call
            let prompt = r#"你是一个植物记录助手。请从用户输入中提取结构化信息。用户输入是关于花花草草的自然语言描述。
请提取以下信息：
- plant_type: 植物种类（例如：黄瓜、土豆、长条盆等）
- action: 动作（例如：记录、培土、翻堆、冒芽等）
- quantity: 数量（整数，如果没有则设为 null）
- notes: 其他备注

请只返回一个 JSON 对象，包含以下字段：plant_type (字符串或null), action (字符串或null), quantity (整数或null), notes (字符串或null)。不要返回其他任何文本。

用户输入：{}"#;

            let request = CreateChatCompletionRequestArgs::default()
                .model("deepseek-chat")
                .messages(vec![
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content("你是一个植物记录助手，请提取结构化信息。")
                        .build()?
                        .into(),
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(prompt.replace("{}", text))
                        .build()?
                        .into(),
                ])
                .build()?;

            let client = self.client.as_ref().expect("Client should exist when not mock");
            let response = client.chat().create(request).await?;
            let content = response.choices[0]
                .message
                .content
                .as_ref()
                .ok_or("No content in response")?;

            Ok(content.to_string())
        }
    }

    // 增强的自然语言解析方法
    pub async fn parse_plant_text_enhanced(
        &self,
        text: &str,
    ) -> Result<ParsedPlantData, Box<dyn std::error::Error + Send + Sync>> {
        if self.is_mock {
            // Mock enhanced parsing
            let parsed_data = self.mock_enhanced_parsing(text);
            Ok(parsed_data)
        } else {
            // Real API call with enhanced prompt
            let parsed_data = self.real_enhanced_parsing(text).await?;
            Ok(parsed_data)
        }
    }

    // 模拟增强解析
    fn mock_enhanced_parsing(&self, text: &str) -> ParsedPlantData {
        let mut parsed = ParsedPlantData {
            event_type: None,
            plant_short_name: None,
            event_date: Some(Utc::now().format("%Y.%m.%d").to_string()),
            quantity_location: None,
            batch: None,
            details: None,
            record_type: None,
            plant_name: None,
            is_germination_report: false,
            is_death_report: false,
            is_cumulative_quantity: false,
            raw_text: text.to_string(),
            confidence: 0.8,
            parsing_errors: Vec::new(),
        };

        // 简单的规则匹配
        if text.contains("黄瓜") {
            parsed.plant_short_name = Some("黄瓜".to_string());
            if text.contains("露白") {
                parsed.event_type = Some("出芽".to_string());
                parsed.quantity_location = Some("1粒".to_string());
                parsed.details = Some("有一粒露白".to_string());
                parsed.is_germination_report = true;
            }
        } else if text.contains("土豆") {
            parsed.plant_short_name = Some("土豆".to_string());
            if text.contains("培土") {
                parsed.event_type = Some("操作".to_string());
                parsed.quantity_location = Some("2棵".to_string());
                parsed.details = Some("有两棵可以培土了".to_string());
            }
        } else if text.contains("长条盆") || text.contains("大长条盆") {
            parsed.plant_short_name = Some("长条盆".to_string());
            if text.contains("冒芽") {
                parsed.event_type = Some("出芽".to_string());
                parsed.quantity_location = Some("1棵".to_string());
                parsed.details = Some("多冒芽一棵".to_string());
                parsed.is_germination_report = true;
            }
        } else if text.contains("堆肥") {
            parsed.plant_name = Some("堆肥".to_string());
            parsed.record_type = Some("操作".to_string());
            if text.contains("翻堆") {
                parsed.details = Some("翻堆了一遍".to_string());
            }
        } else if text.contains("死亡") || text.contains("死了") {
            parsed.event_type = Some("死亡".to_string());
            parsed.is_death_report = true;
        } else if text.contains("今日出芽") || text.contains("今天出芽") {
            parsed.is_germination_report = true;
            parsed.is_cumulative_quantity = true;
        }

        parsed
    }

    // 真实API增强解析
    async fn real_enhanced_parsing(
        &self,
        text: &str,
    ) -> Result<ParsedPlantData, Box<dyn std::error::Error + Send + Sync>> {
        let prompt = r#"你是一个专业的植物记录助手。请从用户输入中提取结构化信息，严格遵循以下业务规则：

需要识别的信息：
1. 事件类型: 必须是以下之一：播种、出芽、假植、移栽、死亡、观察、操作、处理
2. 植物简称: 必须匹配品种档案中的简称（如果用户使用完整名称，尝试匹配常见简称）
3. 日期: YYYY.MM.DD格式（如未指定则使用今天）
4. 数量/位置: 如"8粒"、"6号位"、"1、2、4号杯"
5. 批次: 第一批（辣椒、番茄等）或第二批（瓜类、洛神花等）
6. 详情: 具体描述
7. 记录类型: 操作（用户做了某事）或观察（用户看到了某事）
8. 植物名称: 对于非育苗植物（蓝莓、葡萄、韭菜、堆肥等）

业务规则（必须遵守）：
- 严格区分"操作"（用户做了某事）和"观察"（用户看到了某事）
- 用户问"能不能""要不要"不代表已经做了，只有说"我做了""我种了""我观察到了"才算
- 出芽记录需要标记为is_germination_report=true
- 死亡记录需要标记为is_death_report=true
- 数量为累计总数时需要标记is_cumulative_quantity=true（如"今日出芽的位置"）
- 只记录用户明确说"做了"或明确观察到的内容，不添加任何推测

请返回一个JSON对象，包含以下字段：
- event_type: string或null
- plant_short_name: string或null
- event_date: string (YYYY.MM.DD格式)或null
- quantity_location: string或null
- batch: "第一批"或"第二批"或null
- details: string或null
- record_type: "操作"或"观察"或null
- plant_name: string或null (仅用于非育苗植物)
- is_germination_report: boolean
- is_death_report: boolean
- is_cumulative_quantity: boolean
- confidence: float (0.0-1.0)
- parsing_errors: array of strings

不要返回其他任何文本。

用户输入：{}"#;

        let request = CreateChatCompletionRequestArgs::default()
            .model("deepseek-chat")
            .temperature(0.1)  // 低温度以获得更确定性的输出
            .messages(vec![
                ChatCompletionRequestSystemMessageArgs::default()
                    .content("你是一个严格的植物记录助手，必须遵循所有业务规则，不添加任何推测。")
                    .build()?
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content(prompt.replace("{}", text))
                    .build()?
                    .into(),
            ])
            .build()?;

        let client = self.client.as_ref().expect("Client should exist when not mock");
        let response = client.chat().create(request).await?;
        let content = response.choices[0]
            .message
            .content
            .as_ref()
            .ok_or("No content in response")?;

        // 解析JSON响应
        let mut parsed: ParsedPlantData = serde_json::from_str(content)?;
        parsed.raw_text = text.to_string();

        Ok(parsed)
    }
}