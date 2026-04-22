use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::handlers::AppState;
use crate::models::{NonSeedlingRecord, CreateNonSeedlingRecord, RecordType};
use chrono::NaiveDate;

#[derive(Debug, Deserialize)]
pub struct NonSeedlingQuery {
    pub plant_name: Option<String>,
    pub record_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NonSeedlingListResponse {
    pub success: bool,
    pub message: String,
    pub records: Vec<NonSeedlingRecord>,
}

#[derive(Debug, Serialize)]
pub struct NonSeedlingResponse {
    pub success: bool,
    pub message: String,
    pub record: Option<NonSeedlingRecord>,
}

/// 获取育苗以外植物记录列表
pub async fn list_non_seedling_records(
    State(state): State<Arc<AppState>>,
    Query(query): Query<NonSeedlingQuery>,
) -> (StatusCode, Json<NonSeedlingListResponse>) {
    // Build query
    let mut sql = "SELECT * FROM non_seedling_records".to_string();
    let mut conditions = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(plant_name) = &query.plant_name {
        conditions.push("plant_name = ?");
        params.push(plant_name.clone());
    }

    if let Some(record_type) = &query.record_type {
        conditions.push("record_type = ?");
        params.push(record_type.clone());
    }

    if let Some(start_date) = &query.start_date {
        conditions.push("record_date >= ?");
        params.push(start_date.clone());
    }

    if let Some(end_date) = &query.end_date {
        conditions.push("record_date <= ?");
        params.push(end_date.clone());
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(" ORDER BY record_date DESC");

    // Execute query with dynamic binding
    let mut query = sqlx::query_as::<_, NonSeedlingRecord>(&sql);
    for param in params.iter() {
        query = query.bind(param);
    }

    match query.fetch_all(&state.db_pool).await {
        Ok(records) => {
            let response = NonSeedlingListResponse {
                success: true,
                message: format!("找到 {} 条非育苗植物记录", records.len()),
                records,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch non-seedling records: {}", e);
            let response = NonSeedlingListResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                records: Vec::new(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 创建新的育苗以外植物记录
pub async fn create_non_seedling_record(
    State(state): State<Arc<AppState>>,
    Json(create_record): Json<CreateNonSeedlingRecord>,
) -> (StatusCode, Json<NonSeedlingResponse>) {
    // Insert the record
    match sqlx::query_as::<_, NonSeedlingRecord>(
        "INSERT INTO non_seedling_records (plant_name, record_date, record_type, details, notes) VALUES (?, ?, ?, ?, ?) RETURNING *"
    )
        .bind(&create_record.plant_name)
        .bind(create_record.record_date)
        .bind(create_record.record_type)
        .bind(&create_record.details)
        .bind(create_record.notes)
        .fetch_one(&state.db_pool)
        .await
    {
        Ok(record) => {
            let response = NonSeedlingResponse {
                success: true,
                message: "非育苗植物记录创建成功".to_string(),
                record: Some(record),
            };
            (StatusCode::CREATED, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to create non-seedling record: {}", e);
            let response = NonSeedlingResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                record: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 获取单个育苗以外植物记录
pub async fn get_non_seedling_record(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<NonSeedlingResponse>) {
    match sqlx::query_as::<_, NonSeedlingRecord>("SELECT * FROM non_seedling_records WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(Some(record)) => {
            let response = NonSeedlingResponse {
                success: true,
                message: "找到非育苗植物记录".to_string(),
                record: Some(record),
            };
            (StatusCode::OK, Json(response))
        }
        Ok(None) => {
            let response = NonSeedlingResponse {
                success: false,
                message: format!("未找到ID为 {} 的非育苗植物记录", id),
                record: None,
            };
            (StatusCode::NOT_FOUND, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch non-seedling record: {}", e);
            let response = NonSeedlingResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                record: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateNonSeedlingRecord {
    pub plant_name: Option<String>,
    pub record_date: Option<NaiveDate>,
    pub record_type: Option<RecordType>,
    pub details: Option<String>,
    pub notes: Option<String>,
}

/// 更新育苗以外植物记录
pub async fn update_non_seedling_record(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Json(update_record): Json<UpdateNonSeedlingRecord>,
) -> (StatusCode, Json<NonSeedlingResponse>) {
    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM non_seedling_records WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing non-seedling record: {}", e);
            let response = NonSeedlingResponse {
                success: false,
                message: format!("验证失败: {}", e),
                record: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = NonSeedlingResponse {
            success: false,
            message: format!("未找到ID为 {} 的非育苗植物记录", id),
            record: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 构建动态更新SQL
    let mut updates = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(plant_name) = &update_record.plant_name {
        updates.push("plant_name = ?");
        params.push(plant_name.clone());
    }
    if let Some(record_date) = &update_record.record_date {
        updates.push("record_date = ?");
        params.push(record_date.to_string()); // NaiveDate to string
    }
    if let Some(record_type) = &update_record.record_type {
        updates.push("record_type = ?");
        // RecordType enum to string
        params.push(format!("{:?}", record_type));
    }
    if let Some(details) = &update_record.details {
        updates.push("details = ?");
        params.push(details.clone());
    }
    if let Some(notes) = &update_record.notes {
        updates.push("notes = ?");
        params.push(notes.clone());
    }

    if updates.is_empty() {
        let response = NonSeedlingResponse {
            success: false,
            message: "没有提供要更新的字段".to_string(),
            record: None,
        };
        return (StatusCode::BAD_REQUEST, Json(response));
    }

    // 构建完整SQL
    let sql = format!("UPDATE non_seedling_records SET {} WHERE id = ? RETURNING *", updates.join(", "));
    params.push(id.to_string());

    // 执行更新
    let mut query = sqlx::query_as::<_, NonSeedlingRecord>(&sql);
    for param in params.iter() {
        query = query.bind(param);
    }

    match query.fetch_one(&state.db_pool).await {
        Ok(record) => {
            let response = NonSeedlingResponse {
                success: true,
                message: "非育苗植物记录更新成功".to_string(),
                record: Some(record),
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to update non-seedling record: {}", e);
            let response = NonSeedlingResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                record: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 删除育苗以外植物记录
pub async fn delete_non_seedling_record(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<NonSeedlingResponse>) {
    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM non_seedling_records WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing non-seedling record: {}", e);
            let response = NonSeedlingResponse {
                success: false,
                message: format!("验证失败: {}", e),
                record: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = NonSeedlingResponse {
            success: false,
            message: format!("未找到ID为 {} 的非育苗植物记录", id),
            record: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 执行删除
    match sqlx::query("DELETE FROM non_seedling_records WHERE id = ?")
        .bind(id)
        .execute(&state.db_pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                let response = NonSeedlingResponse {
                    success: true,
                    message: "非育苗植物记录删除成功".to_string(),
                    record: None,
                };
                (StatusCode::OK, Json(response))
            } else {
                // 这应该不会发生，因为我们已经检查过存在性
                let response = NonSeedlingResponse {
                    success: false,
                    message: "删除操作未影响任何记录".to_string(),
                    record: None,
                };
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete non-seedling record: {}", e);
            let response = NonSeedlingResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                record: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}