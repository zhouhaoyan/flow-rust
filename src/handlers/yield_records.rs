use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::handlers::AppState;
use crate::models::{YieldRecord, CreateYieldRecord};
use chrono::NaiveDate;

#[derive(Debug, Deserialize)]
pub struct YieldQuery {
    pub plant_short_name: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct YieldListResponse {
    pub success: bool,
    pub message: String,
    pub yields: Vec<YieldRecord>,
}

#[derive(Debug, Serialize)]
pub struct YieldResponse {
    pub success: bool,
    pub message: String,
    pub yield_record: Option<YieldRecord>,
}

/// 获取产量记录列表
pub async fn list_yield_records(
    State(state): State<Arc<AppState>>,
    Query(query): Query<YieldQuery>,
) -> (StatusCode, Json<YieldListResponse>) {
    // Build query
    let mut sql = "SELECT * FROM yield_records".to_string();
    let mut conditions = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(plant_short_name) = &query.plant_short_name {
        conditions.push("plant_short_name = ?");
        params.push(plant_short_name.clone());
    }

    if let Some(start_date) = &query.start_date {
        conditions.push("harvest_date >= ?");
        params.push(start_date.clone());
    }

    if let Some(end_date) = &query.end_date {
        conditions.push("harvest_date <= ?");
        params.push(end_date.clone());
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(" ORDER BY harvest_date DESC");

    // Execute query with dynamic binding
    let mut query = sqlx::query_as::<_, YieldRecord>(&sql);
    for param in params.iter() {
        query = query.bind(param);
    }

    match query.fetch_all(&state.db_pool).await {
        Ok(yields) => {
            let response = YieldListResponse {
                success: true,
                message: format!("找到 {} 条产量记录", yields.len()),
                yields,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch yield records: {}", e);
            let response = YieldListResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                yields: Vec::new(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 创建新的产量记录
pub async fn create_yield_record(
    State(state): State<Arc<AppState>>,
    Json(create_yield): Json<CreateYieldRecord>,
) -> (StatusCode, Json<YieldResponse>) {
    // Validate plant_short_name exists in plant_archive
    let plant_exists: Option<(i64,)> = match sqlx::query_as("SELECT id FROM plant_archive WHERE short_name = ?")
        .bind(&create_yield.plant_short_name)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to validate plant short name: {}", e);
            let response = YieldResponse {
                success: false,
                message: format!("验证失败: {}", e),
                yield_record: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if plant_exists.is_none() {
        let response = YieldResponse {
            success: false,
            message: format!("植物简称 '{}' 不存在于品种档案中", create_yield.plant_short_name),
            yield_record: None,
        };
        return (StatusCode::BAD_REQUEST, Json(response));
    }

    // Insert the yield record
    match sqlx::query_as::<_, YieldRecord>(
        "INSERT INTO yield_records (plant_short_name, harvest_date, quantity, unit, notes) VALUES (?, ?, ?, ?, ?) RETURNING *"
    )
        .bind(&create_yield.plant_short_name)
        .bind(create_yield.harvest_date)
        .bind(create_yield.quantity)
        .bind(create_yield.unit)
        .bind(create_yield.notes)
        .fetch_one(&state.db_pool)
        .await
    {
        Ok(yield_record) => {
            let response = YieldResponse {
                success: true,
                message: "产量记录创建成功".to_string(),
                yield_record: Some(yield_record),
            };
            (StatusCode::CREATED, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to create yield record: {}", e);
            let response = YieldResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                yield_record: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 获取单个产量记录
pub async fn get_yield_record(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<YieldResponse>) {
    match sqlx::query_as::<_, YieldRecord>("SELECT * FROM yield_records WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(Some(yield_record)) => {
            let response = YieldResponse {
                success: true,
                message: "找到产量记录".to_string(),
                yield_record: Some(yield_record),
            };
            (StatusCode::OK, Json(response))
        }
        Ok(None) => {
            let response = YieldResponse {
                success: false,
                message: format!("未找到ID为 {} 的产量记录", id),
                yield_record: None,
            };
            (StatusCode::NOT_FOUND, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch yield record: {}", e);
            let response = YieldResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                yield_record: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateYieldRecord {
    pub plant_short_name: Option<String>,
    pub harvest_date: Option<NaiveDate>,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
    pub notes: Option<String>,
}

/// 更新产量记录
pub async fn update_yield_record(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Json(update_yield): Json<UpdateYieldRecord>,
) -> (StatusCode, Json<YieldResponse>) {
    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM yield_records WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing yield record: {}", e);
            let response = YieldResponse {
                success: false,
                message: format!("验证失败: {}", e),
                yield_record: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = YieldResponse {
            success: false,
            message: format!("未找到ID为 {} 的产量记录", id),
            yield_record: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 如果提供了新的植物简称，验证其存在于品种档案中
    if let Some(new_short_name) = &update_yield.plant_short_name {
        let plant_exists: Option<(i64,)> = match sqlx::query_as("SELECT id FROM plant_archive WHERE short_name = ?")
            .bind(new_short_name)
            .fetch_optional(&state.db_pool)
            .await
        {
            Ok(row) => row,
            Err(e) => {
                tracing::error!("Failed to validate plant short name: {}", e);
                let response = YieldResponse {
                    success: false,
                    message: format!("验证失败: {}", e),
                    yield_record: None,
                };
                return (StatusCode::BAD_REQUEST, Json(response));
            }
        };

        if plant_exists.is_none() {
            let response = YieldResponse {
                success: false,
                message: format!("植物简称 '{}' 不存在于品种档案中", new_short_name),
                yield_record: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    }

    // 构建动态更新SQL
    let mut updates = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(plant_short_name) = &update_yield.plant_short_name {
        updates.push("plant_short_name = ?");
        params.push(plant_short_name.clone());
    }
    if let Some(harvest_date) = &update_yield.harvest_date {
        updates.push("harvest_date = ?");
        params.push(harvest_date.to_string()); // NaiveDate to string
    }
    if let Some(quantity) = &update_yield.quantity {
        updates.push("quantity = ?");
        params.push(quantity.to_string());
    }
    if let Some(unit) = &update_yield.unit {
        updates.push("unit = ?");
        params.push(unit.clone());
    }
    if let Some(notes) = &update_yield.notes {
        updates.push("notes = ?");
        params.push(notes.clone());
    }

    if updates.is_empty() {
        let response = YieldResponse {
            success: false,
            message: "没有提供要更新的字段".to_string(),
            yield_record: None,
        };
        return (StatusCode::BAD_REQUEST, Json(response));
    }

    // 构建完整SQL
    let sql = format!("UPDATE yield_records SET {} WHERE id = ? RETURNING *", updates.join(", "));
    params.push(id.to_string());

    // 执行更新
    let mut query = sqlx::query_as::<_, YieldRecord>(&sql);
    for param in params.iter() {
        query = query.bind(param);
    }

    match query.fetch_one(&state.db_pool).await {
        Ok(yield_record) => {
            let response = YieldResponse {
                success: true,
                message: "产量记录更新成功".to_string(),
                yield_record: Some(yield_record),
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to update yield record: {}", e);
            let response = YieldResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                yield_record: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 删除产量记录
pub async fn delete_yield_record(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<YieldResponse>) {
    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM yield_records WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing yield record: {}", e);
            let response = YieldResponse {
                success: false,
                message: format!("验证失败: {}", e),
                yield_record: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = YieldResponse {
            success: false,
            message: format!("未找到ID为 {} 的产量记录", id),
            yield_record: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 执行删除
    match sqlx::query("DELETE FROM yield_records WHERE id = ?")
        .bind(id)
        .execute(&state.db_pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                let response = YieldResponse {
                    success: true,
                    message: "产量记录删除成功".to_string(),
                    yield_record: None,
                };
                (StatusCode::OK, Json(response))
            } else {
                // 这应该不会发生，因为我们已经检查过存在性
                let response = YieldResponse {
                    success: false,
                    message: "删除操作未影响任何记录".to_string(),
                    yield_record: None,
                };
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete yield record: {}", e);
            let response = YieldResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                yield_record: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}