use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::handlers::AppState;
use crate::models::{GrowthLog, CreateGrowthLog, EventType, deserialize_optional_naive_date};
use chrono::NaiveDate;

#[derive(Debug, Deserialize)]
pub struct GrowthLogQuery {
    pub batch: Option<String>, // "batch1" or "batch2"
    pub plant_short_name: Option<String>,
    pub event_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GrowthLogListResponse {
    pub success: bool,
    pub message: String,
    pub logs: Vec<GrowthLog>,
}

#[derive(Debug, Serialize)]
pub struct GrowthLogResponse {
    pub success: bool,
    pub message: String,
    pub log: Option<GrowthLog>,
}

/// 获取生长日志列表
pub async fn list_growth_logs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GrowthLogQuery>,
) -> (StatusCode, Json<GrowthLogListResponse>) {
    // Determine which table to query
    let table_name = match query.batch.as_deref() {
        Some("batch1") => "growth_log_batch1",
        Some("batch2") => "growth_log_batch2",
        _ => "growth_log_batch1", // default to batch1
    };

    // Build query dynamically
    let mut sql = format!("SELECT * FROM {}", table_name);
    let mut conditions = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(plant_short_name) = &query.plant_short_name {
        conditions.push("plant_short_name = ?");
        params.push(plant_short_name.clone());
    }

    if let Some(event_type) = &query.event_type {
        conditions.push("event_type = ?");
        params.push(event_type.clone());
    }

    if let Some(start_date) = &query.start_date {
        conditions.push("event_date >= ?");
        params.push(start_date.clone());
    }

    if let Some(end_date) = &query.end_date {
        conditions.push("event_date <= ?");
        params.push(end_date.clone());
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    // Add sorting: plant_short_name > event_date > id
    sql.push_str(" ORDER BY plant_short_name, event_date, id");

    // Execute query with dynamic binding
    let mut query = sqlx::query_as::<_, GrowthLog>(&sql);
    for param in params.iter() {
        query = query.bind(param);
    }

    match query.fetch_all(&state.db_pool).await {
        Ok(logs) => {
            let response = GrowthLogListResponse {
                success: true,
                message: format!("找到 {} 条生长日志", logs.len()),
                logs,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch growth logs: {}", e);
            let response = GrowthLogListResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                logs: Vec::new(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 创建新的生长日志记录
pub async fn create_growth_log(
    State(state): State<Arc<AppState>>,
    Json(create_log): Json<CreateGrowthLog>,
) -> (StatusCode, Json<GrowthLogResponse>) {
    // Determine which table to insert into based on plant type
    // This is a simplification - in reality we'd need to know the batch
    let table_name = "growth_log_batch1"; // Default to batch1

    // First, validate that plant_short_name exists in plant_archive
    let plant_exists: Option<(i64,)> = match sqlx::query_as("SELECT id FROM plant_archive WHERE short_name = ?")
        .bind(&create_log.plant_short_name)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to validate plant short name: {}", e);
            let response = GrowthLogResponse {
                success: false,
                message: format!("验证失败: {}", e),
                log: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if plant_exists.is_none() {
        let response = GrowthLogResponse {
            success: false,
            message: format!("植物简称 '{}' 不存在于品种档案中", create_log.plant_short_name),
            log: None,
        };
        return (StatusCode::BAD_REQUEST, Json(response));
    }

    // Insert the growth log
    let sql = format!(
        "INSERT INTO {} (plant_short_name, event_date, event_type, quantity_location, details) VALUES (?, ?, ?, ?, ?) RETURNING *",
        table_name
    );

    match sqlx::query_as::<_, GrowthLog>(&sql)
        .bind(&create_log.plant_short_name)
        .bind(create_log.event_date)
        .bind(create_log.event_type)
        .bind(create_log.quantity_location)
        .bind(create_log.details)
        .fetch_one(&state.db_pool)
        .await
    {
        Ok(log) => {
            let response = GrowthLogResponse {
                success: true,
                message: "生长日志创建成功".to_string(),
                log: Some(log),
            };
            (StatusCode::CREATED, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to create growth log: {}", e);
            let response = GrowthLogResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                log: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 获取单个生长日志记录
pub async fn get_growth_log(
    Path((batch, id)): Path<(String, i64)>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<GrowthLogResponse>) {
    let table_name = match batch.as_str() {
        "batch1" => "growth_log_batch1",
        "batch2" => "growth_log_batch2",
        _ => {
            let response = GrowthLogResponse {
                success: false,
                message: "无效的批次名称，请使用 'batch1' 或 'batch2'".to_string(),
                log: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    let sql = format!("SELECT * FROM {} WHERE id = ?", table_name);

    match sqlx::query_as::<_, GrowthLog>(&sql)
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(Some(log)) => {
            let response = GrowthLogResponse {
                success: true,
                message: "找到生长日志".to_string(),
                log: Some(log),
            };
            (StatusCode::OK, Json(response))
        }
        Ok(None) => {
            let response = GrowthLogResponse {
                success: false,
                message: format!("未找到ID为 {} 的生长日志", id),
                log: None,
            };
            (StatusCode::NOT_FOUND, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch growth log: {}", e);
            let response = GrowthLogResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                log: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct UpdateGrowthLog {
    pub plant_short_name: Option<String>,
    #[serde(deserialize_with = "deserialize_optional_naive_date")]
    pub event_date: Option<NaiveDate>,
    pub event_type: Option<EventType>,
    pub quantity_location: Option<String>,
    pub details: Option<String>,
}

/// 更新生长日志记录
pub async fn update_growth_log(
    Path((batch, id)): Path<(String, i64)>,
    State(state): State<Arc<AppState>>,
    Json(update_log): Json<UpdateGrowthLog>,
) -> (StatusCode, Json<GrowthLogResponse>) {
    // 验证批次名称
    let table_name = match batch.as_str() {
        "batch1" => "growth_log_batch1",
        "batch2" => "growth_log_batch2",
        _ => {
            let response = GrowthLogResponse {
                success: false,
                message: "无效的批次名称，请使用 'batch1' 或 'batch2'".to_string(),
                log: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as(&format!("SELECT id FROM {} WHERE id = ?", table_name))
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing growth log: {}", e);
            let response = GrowthLogResponse {
                success: false,
                message: format!("验证失败: {}", e),
                log: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = GrowthLogResponse {
            success: false,
            message: format!("未找到ID为 {} 的生长日志", id),
            log: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 如果提供了新的植物简称，验证其存在于品种档案中
    if let Some(new_short_name) = &update_log.plant_short_name {
        let plant_exists: Option<(i64,)> = match sqlx::query_as("SELECT id FROM plant_archive WHERE short_name = ?")
            .bind(new_short_name)
            .fetch_optional(&state.db_pool)
            .await
        {
            Ok(row) => row,
            Err(e) => {
                tracing::error!("Failed to validate plant short name: {}", e);
                let response = GrowthLogResponse {
                    success: false,
                    message: format!("验证失败: {}", e),
                    log: None,
                };
                return (StatusCode::BAD_REQUEST, Json(response));
            }
        };

        if plant_exists.is_none() {
            let response = GrowthLogResponse {
                success: false,
                message: format!("植物简称 '{}' 不存在于品种档案中", new_short_name),
                log: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    }

    // 构建动态更新SQL
    let mut updates = Vec::new();
    let mut has_updates = false;

    if update_log.plant_short_name.is_some() {
        updates.push("plant_short_name = ?");
        has_updates = true;
    }
    if update_log.event_date.is_some() {
        updates.push("event_date = ?");
        has_updates = true;
    }
    if update_log.event_type.is_some() {
        updates.push("event_type = ?");
        has_updates = true;
    }
    if update_log.quantity_location.is_some() {
        updates.push("quantity_location = ?");
        has_updates = true;
    }
    if update_log.details.is_some() {
        updates.push("details = ?");
        has_updates = true;
    }

    if !has_updates {
        let response = GrowthLogResponse {
            success: false,
            message: "没有提供要更新的字段".to_string(),
            log: None,
        };
        return (StatusCode::BAD_REQUEST, Json(response));
    }

    // 构建完整SQL
    let sql = format!("UPDATE {} SET {} WHERE id = ? RETURNING *", table_name, updates.join(", "));

    // 执行更新，按顺序绑定参数
    let mut query = sqlx::query_as::<_, GrowthLog>(&sql);
    if let Some(plant_short_name) = &update_log.plant_short_name {
        query = query.bind(plant_short_name);
    }
    if let Some(event_date) = &update_log.event_date {
        query = query.bind(event_date);
    }
    if let Some(event_type) = &update_log.event_type {
        query = query.bind(event_type);
    }
    if let Some(quantity_location) = &update_log.quantity_location {
        query = query.bind(quantity_location);
    }
    if let Some(details) = &update_log.details {
        query = query.bind(details);
    }
    query = query.bind(id);

    match query.fetch_one(&state.db_pool).await {
        Ok(log) => {
            let response = GrowthLogResponse {
                success: true,
                message: "生长日志更新成功".to_string(),
                log: Some(log),
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to update growth log: {}", e);
            let response = GrowthLogResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                log: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 删除生长日志记录
pub async fn delete_growth_log(
    Path((batch, id)): Path<(String, i64)>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<GrowthLogResponse>) {
    // 验证批次名称
    let table_name = match batch.as_str() {
        "batch1" => "growth_log_batch1",
        "batch2" => "growth_log_batch2",
        _ => {
            let response = GrowthLogResponse {
                success: false,
                message: "无效的批次名称，请使用 'batch1' 或 'batch2'".to_string(),
                log: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as(&format!("SELECT id FROM {} WHERE id = ?", table_name))
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing growth log: {}", e);
            let response = GrowthLogResponse {
                success: false,
                message: format!("验证失败: {}", e),
                log: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = GrowthLogResponse {
            success: false,
            message: format!("未找到ID为 {} 的生长日志", id),
            log: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 执行删除
    match sqlx::query(&format!("DELETE FROM {} WHERE id = ?", table_name))
        .bind(id)
        .execute(&state.db_pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                let response = GrowthLogResponse {
                    success: true,
                    message: "生长日志删除成功".to_string(),
                    log: None,
                };
                (StatusCode::OK, Json(response))
            } else {
                // 这应该不会发生，因为我们已经检查过存在性
                let response = GrowthLogResponse {
                    success: false,
                    message: "删除操作未影响任何记录".to_string(),
                    log: None,
                };
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete growth log: {}", e);
            let response = GrowthLogResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                log: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}