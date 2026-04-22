use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::handlers::AppState;
use crate::models::{PlantArchive, CreatePlantArchive};

#[derive(Debug, Serialize)]
pub struct PlantArchiveListResponse {
    pub success: bool,
    pub message: String,
    pub archives: Vec<PlantArchive>,
}

#[derive(Debug, Serialize)]
pub struct PlantArchiveResponse {
    pub success: bool,
    pub message: String,
    pub archive: Option<PlantArchive>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlantArchive {
    pub short_name: Option<String>,
    pub full_name: Option<String>,
    pub category: Option<String>,
    pub variety_type: Option<String>,
    pub height_habit: Option<String>,
    pub fruit_features: Option<String>,
    pub taste_usage: Option<String>,
    pub estimated_yield: Option<String>,
    pub notes: Option<String>,
}

/// 获取所有品种档案
pub async fn list_plant_archives(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<PlantArchiveListResponse>) {
    match sqlx::query_as::<_, PlantArchive>("SELECT * FROM plant_archive ORDER BY short_name")
        .fetch_all(&state.db_pool)
        .await
    {
        Ok(archives) => {
            let response = PlantArchiveListResponse {
                success: true,
                message: format!("找到 {} 个品种", archives.len()),
                archives,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch plant archives: {}", e);
            let response = PlantArchiveListResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                archives: Vec::new(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 创建新品种档案
pub async fn create_plant_archive(
    State(state): State<Arc<AppState>>,
    Json(create_archive): Json<CreatePlantArchive>,
) -> (StatusCode, Json<PlantArchiveResponse>) {
    // 检查简称是否已存在
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM plant_archive WHERE short_name = ?")
        .bind(&create_archive.short_name)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing plant archive: {}", e);
            let response = PlantArchiveResponse {
                success: false,
                message: format!("验证失败: {}", e),
                archive: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_some() {
        let response = PlantArchiveResponse {
            success: false,
            message: format!("简称 '{}' 已存在", create_archive.short_name),
            archive: None,
        };
        return (StatusCode::BAD_REQUEST, Json(response));
    }

    // 插入新品种档案
    match sqlx::query_as::<_, PlantArchive>(
        "INSERT INTO plant_archive (short_name, full_name, category, variety_type, height_habit, fruit_features, taste_usage, estimated_yield, notes) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING *"
    )
        .bind(&create_archive.short_name)
        .bind(create_archive.full_name)
        .bind(create_archive.category)
        .bind(create_archive.variety_type)
        .bind(create_archive.height_habit)
        .bind(create_archive.fruit_features)
        .bind(create_archive.taste_usage)
        .bind(create_archive.estimated_yield)
        .bind(create_archive.notes)
        .fetch_one(&state.db_pool)
        .await
    {
        Ok(archive) => {
            let response = PlantArchiveResponse {
                success: true,
                message: "品种档案创建成功".to_string(),
                archive: Some(archive),
            };
            (StatusCode::CREATED, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to create plant archive: {}", e);
            let response = PlantArchiveResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                archive: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 获取单个品种档案
pub async fn get_plant_archive(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<PlantArchiveResponse>) {
    match sqlx::query_as::<_, PlantArchive>("SELECT * FROM plant_archive WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(Some(archive)) => {
            let response = PlantArchiveResponse {
                success: true,
                message: "找到品种档案".to_string(),
                archive: Some(archive),
            };
            (StatusCode::OK, Json(response))
        }
        Ok(None) => {
            let response = PlantArchiveResponse {
                success: false,
                message: format!("未找到ID为 {} 的品种档案", id),
                archive: None,
            };
            (StatusCode::NOT_FOUND, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch plant archive: {}", e);
            let response = PlantArchiveResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                archive: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 更新品种档案
pub async fn update_plant_archive(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Json(update_archive): Json<UpdatePlantArchive>,
) -> (StatusCode, Json<PlantArchiveResponse>) {
    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM plant_archive WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing plant archive: {}", e);
            let response = PlantArchiveResponse {
                success: false,
                message: format!("验证失败: {}", e),
                archive: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = PlantArchiveResponse {
            success: false,
            message: format!("未找到ID为 {} 的品种档案", id),
            archive: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 如果提供了新的简称，检查是否与其他记录冲突
    if let Some(new_short_name) = &update_archive.short_name {
        let conflict: Option<(i64,)> = match sqlx::query_as("SELECT id FROM plant_archive WHERE short_name = ? AND id != ?")
            .bind(new_short_name)
            .bind(id)
            .fetch_optional(&state.db_pool)
            .await
        {
            Ok(row) => row,
            Err(e) => {
                tracing::error!("Failed to check short name conflict: {}", e);
                let response = PlantArchiveResponse {
                    success: false,
                    message: format!("验证失败: {}", e),
                    archive: None,
                };
                return (StatusCode::BAD_REQUEST, Json(response));
            }
        };

        if conflict.is_some() {
            let response = PlantArchiveResponse {
                success: false,
                message: format!("简称 '{}' 已被其他记录使用", new_short_name),
                archive: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    }

    // 构建动态更新SQL
    let mut updates = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(short_name) = &update_archive.short_name {
        updates.push("short_name = ?");
        params.push(short_name.clone());
    }
    if let Some(full_name) = &update_archive.full_name {
        updates.push("full_name = ?");
        params.push(full_name.clone());
    }
    if let Some(category) = &update_archive.category {
        updates.push("category = ?");
        params.push(category.clone());
    }
    if let Some(variety_type) = &update_archive.variety_type {
        updates.push("variety_type = ?");
        params.push(variety_type.clone());
    }
    if let Some(height_habit) = &update_archive.height_habit {
        updates.push("height_habit = ?");
        params.push(height_habit.clone());
    }
    if let Some(fruit_features) = &update_archive.fruit_features {
        updates.push("fruit_features = ?");
        params.push(fruit_features.clone());
    }
    if let Some(taste_usage) = &update_archive.taste_usage {
        updates.push("taste_usage = ?");
        params.push(taste_usage.clone());
    }
    if let Some(estimated_yield) = &update_archive.estimated_yield {
        updates.push("estimated_yield = ?");
        params.push(estimated_yield.clone());
    }
    if let Some(notes) = &update_archive.notes {
        updates.push("notes = ?");
        params.push(notes.clone());
    }

    if updates.is_empty() {
        let response = PlantArchiveResponse {
            success: false,
            message: "没有提供要更新的字段".to_string(),
            archive: None,
        };
        return (StatusCode::BAD_REQUEST, Json(response));
    }

    // 构建完整SQL
    let sql = format!("UPDATE plant_archive SET {}, updated_at = CURRENT_TIMESTAMP WHERE id = ? RETURNING *", updates.join(", "));
    params.push(id.to_string());

    // 执行更新
    let mut query = sqlx::query_as::<_, PlantArchive>(&sql);
    for param in params.iter() {
        query = query.bind(param);
    }

    match query.fetch_one(&state.db_pool).await {
        Ok(archive) => {
            let response = PlantArchiveResponse {
                success: true,
                message: "品种档案更新成功".to_string(),
                archive: Some(archive),
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to update plant archive: {}", e);
            let response = PlantArchiveResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                archive: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 删除品种档案
pub async fn delete_plant_archive(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<PlantArchiveResponse>) {
    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM plant_archive WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing plant archive: {}", e);
            let response = PlantArchiveResponse {
                success: false,
                message: format!("验证失败: {}", e),
                archive: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = PlantArchiveResponse {
            success: false,
            message: format!("未找到ID为 {} 的品种档案", id),
            archive: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 执行删除
    match sqlx::query("DELETE FROM plant_archive WHERE id = ?")
        .bind(id)
        .execute(&state.db_pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                let response = PlantArchiveResponse {
                    success: true,
                    message: "品种档案删除成功".to_string(),
                    archive: None,
                };
                (StatusCode::OK, Json(response))
            } else {
                // 这应该不会发生，因为我们已经检查过存在性
                let response = PlantArchiveResponse {
                    success: false,
                    message: "删除操作未影响任何记录".to_string(),
                    archive: None,
                };
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete plant archive: {}", e);
            let response = PlantArchiveResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                archive: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}