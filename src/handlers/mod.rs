use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use crate::deepseek::DeepSeekClient;
use crate::store::{Store, PlantData};
use crate::models::ParsedPlantData;
use crate::validators;
use crate::business_rules::{GerminationTracker, DeathRecorder, QuantityCalculator, EventClassifier};

pub mod plant_archive;
pub mod growth_logs;
pub mod statistics;
pub mod yield_records;
pub mod non_seedling_records;
pub mod fertilizer_materials;
pub mod container_sizes;
pub mod todo_reminders;
#[derive(Debug, Deserialize)]
pub struct PlantRecordRequest {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct PlantRecordResponse {
    pub success: bool,
    pub message: String,
    pub record_id: Option<i64>,
    pub data: Option<PlantData>,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmRequest {
    pub confirm: bool,
}

pub struct AppState {
    pub deepseek_client: DeepSeekClient,
    pub store: Arc<Store>,
    pub db_pool: SqlitePool,
}

#[axum::debug_handler]
pub async fn record_plant_data(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PlantRecordRequest>,
) -> (StatusCode, Json<PlantRecordResponse>) {
    // First try enhanced parsing
    let enhanced_result = state
        .deepseek_client
        .parse_plant_text_enhanced(&request.text)
        .await;

    match enhanced_result {
        Ok(mut parsed_data) => {
            // Set raw_text if not already set
            parsed_data.raw_text = request.text.clone();

            // Run validation
            let validation_result = validators::validate_parsed_data(
                &state.db_pool,
                parsed_data.event_type.as_deref(),
                parsed_data.plant_short_name.as_deref(),
                parsed_data.event_date.as_deref(),
                parsed_data.quantity_location.as_deref(),
                parsed_data.batch.as_deref(),
                parsed_data.record_type.as_deref(),
                parsed_data.is_germination_report,
                parsed_data.is_death_report,
                parsed_data.is_cumulative_quantity,
                parsed_data.details.as_deref(),
                &parsed_data.raw_text,
            ).await;

            // Add validation errors to parsing errors
            if !validation_result.errors.is_empty() {
                parsed_data.parsing_errors.extend(
                    validation_result.errors.iter().map(|e| format!("{}: {}", e.field, e.message))
                );
            }
            if !validation_result.warnings.is_empty() {
                parsed_data.parsing_errors.extend(
                    validation_result.warnings.iter().map(|e| format!("警告 {}: {}", e.field, e.message))
                );
            }
            if !validation_result.infos.is_empty() {
                parsed_data.parsing_errors.extend(
                    validation_result.infos.iter().map(|e| format!("信息 {}: {}", e.field, e.message))
                );
            }

            // Business rules validation

            // Initialize business rule validators
            let germination_tracker = GerminationTracker::new(state.db_pool.clone());
            let death_recorder = DeathRecorder::new(state.db_pool.clone());
            let quantity_calculator = QuantityCalculator::new(state.db_pool.clone());
            let event_classifier = EventClassifier::new();

            // Run germination tracker validation
            match germination_tracker.validate_germination_report(&parsed_data).await {
                Ok((warnings, errors)) => {
                    for warning in warnings {
                        parsed_data.parsing_errors.push(format!("发芽跟踪: {}", warning));
                    }
                    for error in errors {
                        parsed_data.parsing_errors.push(format!("发芽跟踪错误: {}", error));
                    }
                }
                Err(e) => {
                    parsed_data.parsing_errors.push(format!("发芽跟踪验证失败: {}", e));
                }
            }

            // Run death recorder validation
            match death_recorder.validate_death_record(&parsed_data).await {
                Ok((warnings, errors)) => {
                    for warning in warnings {
                        parsed_data.parsing_errors.push(format!("死亡记录: {}", warning));
                    }
                    for error in errors {
                        parsed_data.parsing_errors.push(format!("死亡记录错误: {}", error));
                    }
                }
                Err(e) => {
                    parsed_data.parsing_errors.push(format!("死亡记录验证失败: {}", e));
                }
            }

            // Run quantity calculator validation
            match quantity_calculator.analyze_quantity(&parsed_data).await {
                Ok((warnings, errors)) => {
                    for warning in warnings {
                        parsed_data.parsing_errors.push(format!("数量计算: {}", warning));
                    }
                    for error in errors {
                        parsed_data.parsing_errors.push(format!("数量计算错误: {}", error));
                    }
                }
                Err(e) => {
                    parsed_data.parsing_errors.push(format!("数量计算验证失败: {}", e));
                }
            }

            // Run quantity format validation
            match quantity_calculator.validate_quantity_format(&parsed_data) {
                Ok((warnings, errors)) => {
                    for warning in warnings {
                        parsed_data.parsing_errors.push(format!("数量格式: {}", warning));
                    }
                    for error in errors {
                        parsed_data.parsing_errors.push(format!("数量格式错误: {}", error));
                    }
                }
                Err(e) => {
                    parsed_data.parsing_errors.push(format!("数量格式验证失败: {}", e));
                }
            }

            // Run event classifier validation
            match event_classifier.classify_event(&parsed_data) {
                Ok((warnings, errors)) => {
                    for warning in warnings {
                        parsed_data.parsing_errors.push(format!("事件分类: {}", warning));
                    }
                    for error in errors {
                        parsed_data.parsing_errors.push(format!("事件分类错误: {}", error));
                    }
                }
                Err(e) => {
                    parsed_data.parsing_errors.push(format!("事件分类验证失败: {}", e));
                }
            }

            // Run event type rules validation
            match event_classifier.validate_event_type_rules(&parsed_data) {
                Ok((warnings, errors)) => {
                    for warning in warnings {
                        parsed_data.parsing_errors.push(format!("事件类型规则: {}", warning));
                    }
                    for error in errors {
                        parsed_data.parsing_errors.push(format!("事件类型规则错误: {}", error));
                    }
                }
                Err(e) => {
                    parsed_data.parsing_errors.push(format!("事件类型规则验证失败: {}", e));
                }
            }

            // Store record
            let record_id = match state.store.add_record(request.text, Some(parsed_data)).await {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!("Failed to store record: {}", e);
                    let response = PlantRecordResponse {
                        success: false,
                        message: format!("Database error: {}", e),
                        record_id: None,
                        data: None,
                    };
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(response));
                }
            };

            let message = if !validation_result.errors.is_empty() {
                format!("解析完成但有{}个错误，请确认。", validation_result.errors.len())
            } else if !validation_result.warnings.is_empty() {
                format!("解析完成但有{}个警告，请确认。", validation_result.warnings.len())
            } else {
                "记录已接收。请确认。".to_string()
            };

            let response = PlantRecordResponse {
                success: validation_result.errors.is_empty(),
                message,
                record_id: Some(record_id),
                data: None,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            // Fallback to old parsing method
            tracing::warn!("Enhanced parsing failed, falling back to old method: {}", e);

            let parsed_result = state
                .deepseek_client
                .parse_plant_text(&request.text)
                .await
                .map_err(|e| e.to_string());

            match parsed_result {
                Ok(json_str) => {
                    // Try to parse JSON into PlantData
                    let parsed_data = match serde_json::from_str::<PlantData>(&json_str) {
                        Ok(data) => {
                            tracing::info!("Parsed plant data (legacy): {:?}", data);
                            Some(data)
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse JSON from DeepSeek: {}", e);
                            // Fallback: store raw JSON as notes
                            Some(PlantData {
                                plant_type: None,
                                action: None,
                                quantity: None,
                                notes: Some(json_str.clone()),
                            })
                        }
                    };

                    // Convert PlantData to ParsedPlantData for storage
                    let parsed_plant_data = if let Some(data) = parsed_data {
                        Some(ParsedPlantData {
                            event_type: None,
                            plant_short_name: data.plant_type,
                            event_date: None,
                            quantity_location: None,
                            batch: None,
                            details: data.notes.clone(),
                            record_type: None,
                            plant_name: None,
                            is_germination_report: false,
                            is_death_report: false,
                            is_cumulative_quantity: false,
                            raw_text: request.text.clone(),
                            confidence: 0.5,
                            parsing_errors: vec!["使用旧解析方法".to_string()],
                        })
                    } else {
                        None
                    };

                    // Store record
                    let record_id = match state.store.add_record(request.text, parsed_plant_data).await {
                        Ok(id) => id,
                        Err(e) => {
                            tracing::error!("Failed to store record: {}", e);
                            let response = PlantRecordResponse {
                                success: false,
                                message: format!("Database error: {}", e),
                                record_id: None,
                                data: None,
                            };
                            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response));
                        }
                    };

                    let response = PlantRecordResponse {
                        success: true,
                        message: "记录已接收（使用旧解析方法）。请确认。".to_string(),
                        record_id: Some(record_id),
                        data: None,
                    };
                    (StatusCode::OK, Json(response))
                }
                Err(e) => {
                    tracing::error!("Failed to parse text: {}", e);
                    let response = PlantRecordResponse {
                        success: false,
                        message: format!("错误: {}", e),
                        record_id: None,
                        data: None,
                    };
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                }
            }
        }
    }
}

#[axum::debug_handler]
pub async fn confirm_record(
    Path(record_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<ConfirmRequest>,
) -> (StatusCode, Json<PlantRecordResponse>) {
    if request.confirm {
        let success = match state.store.confirm_record(record_id).await {
            Ok(success) => success,
            Err(e) => {
                tracing::error!("Failed to confirm record: {}", e);
                let response = PlantRecordResponse {
                    success: false,
                    message: format!("Database error: {}", e),
                    record_id: None,
                    data: None,
                };
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(response));
            }
        };
        if success {
            let response = PlantRecordResponse {
                success: true,
                message: "Record confirmed.".to_string(),
                record_id: Some(record_id),
                data: None,
            };
            (StatusCode::OK, Json(response))
        } else {
            let response = PlantRecordResponse {
                success: false,
                message: "Record not found.".to_string(),
                record_id: None,
                data: None,
            };
            (StatusCode::NOT_FOUND, Json(response))
        }
    } else {
        // User rejected the change
        let response = PlantRecordResponse {
            success: false,
            message: "Record rejected.".to_string(),
            record_id: Some(record_id),
            data: None,
        };
        (StatusCode::OK, Json(response))
    }
}

#[derive(Debug, Serialize)]
pub struct PlantRecordsResponse {
    pub success: bool,
    pub message: String,
    pub records: Vec<crate::store::PlantRecord>,
}

#[axum::debug_handler]
pub async fn get_records(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<PlantRecordsResponse>) {
    match state.store.list_records().await {
        Ok(records) => {
            let response = PlantRecordsResponse {
                success: true,
                message: format!("Found {} records", records.len()),
                records,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch records: {}", e);
            let response = PlantRecordsResponse {
                success: false,
                message: format!("Error: {}", e),
                records: Vec::new(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}