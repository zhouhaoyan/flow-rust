use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::handlers::AppState;
use crate::models::{FertilizerMaterial, CreateFertilizerMaterial};

#[derive(Debug, Deserialize)]
pub struct FertilizerQuery {
    pub category: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FertilizerListResponse {
    pub success: bool,
    pub message: String,
    pub materials: Vec<FertilizerMaterial>,
}

#[derive(Debug, Serialize)]
pub struct FertilizerResponse {
    pub success: bool,
    pub message: String,
    pub material: Option<FertilizerMaterial>,
}

/// 获取肥料与基质信息列表
pub async fn list_fertilizer_materials(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FertilizerQuery>,
) -> (StatusCode, Json<FertilizerListResponse>) {
    // Build query
    let mut sql = "SELECT * FROM fertilizer_materials".to_string();
    let mut conditions = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(category) = &query.category {
        conditions.push("category = ?");
        params.push(category.clone());
    }

    if let Some(name) = &query.name {
        conditions.push("name LIKE ?");
        params.push(format!("%{}%", name));
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(" ORDER BY name");

    // Execute query with dynamic binding
    let mut query = sqlx::query_as::<_, FertilizerMaterial>(&sql);
    for param in params.iter() {
        query = query.bind(param);
    }

    match query.fetch_all(&state.db_pool).await {
        Ok(materials) => {
            let response = FertilizerListResponse {
                success: true,
                message: format!("找到 {} 条肥料/基质记录", materials.len()),
                materials,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch fertilizer materials: {}", e);
            let response = FertilizerListResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                materials: Vec::new(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 创建新的肥料与基质信息
pub async fn create_fertilizer_material(
    State(state): State<Arc<AppState>>,
    Json(create_material): Json<CreateFertilizerMaterial>,
) -> (StatusCode, Json<FertilizerResponse>) {
    // Check if material with same name already exists
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM fertilizer_materials WHERE name = ?")
        .bind(&create_material.name)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing material: {}", e);
            let response = FertilizerResponse {
                success: false,
                message: format!("验证失败: {}", e),
                material: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_some() {
        let response = FertilizerResponse {
            success: false,
            message: format!("名称 '{}' 已存在", create_material.name),
            material: None,
        };
        return (StatusCode::BAD_REQUEST, Json(response));
    }

    // Insert the material
    match sqlx::query_as::<_, FertilizerMaterial>(
        "INSERT INTO fertilizer_materials (name, category, description, usage_instructions, notes) VALUES (?, ?, ?, ?, ?) RETURNING *"
    )
        .bind(&create_material.name)
        .bind(create_material.category)
        .bind(create_material.description)
        .bind(create_material.usage_instructions)
        .bind(create_material.notes)
        .fetch_one(&state.db_pool)
        .await
    {
        Ok(material) => {
            let response = FertilizerResponse {
                success: true,
                message: "肥料/基质信息创建成功".to_string(),
                material: Some(material),
            };
            (StatusCode::CREATED, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to create fertilizer material: {}", e);
            let response = FertilizerResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                material: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 获取单个肥料与基质信息
pub async fn get_fertilizer_material(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<FertilizerResponse>) {
    match sqlx::query_as::<_, FertilizerMaterial>("SELECT * FROM fertilizer_materials WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(Some(material)) => {
            let response = FertilizerResponse {
                success: true,
                message: "找到肥料/基质信息".to_string(),
                material: Some(material),
            };
            (StatusCode::OK, Json(response))
        }
        Ok(None) => {
            let response = FertilizerResponse {
                success: false,
                message: format!("未找到ID为 {} 的肥料/基质信息", id),
                material: None,
            };
            (StatusCode::NOT_FOUND, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch fertilizer material: {}", e);
            let response = FertilizerResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                material: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateFertilizerMaterial {
    pub name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub usage_instructions: Option<String>,
    pub notes: Option<String>,
}

/// 更新肥料与基质信息
pub async fn update_fertilizer_material(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Json(update_material): Json<UpdateFertilizerMaterial>,
) -> (StatusCode, Json<FertilizerResponse>) {
    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM fertilizer_materials WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing fertilizer material: {}", e);
            let response = FertilizerResponse {
                success: false,
                message: format!("验证失败: {}", e),
                material: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = FertilizerResponse {
            success: false,
            message: format!("未找到ID为 {} 的肥料/基质信息", id),
            material: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 如果提供了新的名称，检查是否与其他记录冲突
    if let Some(new_name) = &update_material.name {
        let name_exists: Option<(i64,)> = match sqlx::query_as("SELECT id FROM fertilizer_materials WHERE name = ? AND id != ?")
            .bind(new_name)
            .bind(id)
            .fetch_optional(&state.db_pool)
            .await
        {
            Ok(row) => row,
            Err(e) => {
                tracing::error!("Failed to check name conflict: {}", e);
                let response = FertilizerResponse {
                    success: false,
                    message: format!("名称冲突验证失败: {}", e),
                    material: None,
                };
                return (StatusCode::BAD_REQUEST, Json(response));
            }
        };

        if name_exists.is_some() {
            let response = FertilizerResponse {
                success: false,
                message: format!("名称 '{}' 已存在", new_name),
                material: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    }

    // 构建动态更新SQL
    let mut updates = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(name) = &update_material.name {
        updates.push("name = ?");
        params.push(name.clone());
    }
    if let Some(category) = &update_material.category {
        updates.push("category = ?");
        params.push(category.clone());
    }
    if let Some(description) = &update_material.description {
        updates.push("description = ?");
        params.push(description.clone());
    }
    if let Some(usage_instructions) = &update_material.usage_instructions {
        updates.push("usage_instructions = ?");
        params.push(usage_instructions.clone());
    }
    if let Some(notes) = &update_material.notes {
        updates.push("notes = ?");
        params.push(notes.clone());
    }

    if updates.is_empty() {
        let response = FertilizerResponse {
            success: false,
            message: "没有提供要更新的字段".to_string(),
            material: None,
        };
        return (StatusCode::BAD_REQUEST, Json(response));
    }

    // 构建完整SQL
    let sql = format!("UPDATE fertilizer_materials SET {} WHERE id = ? RETURNING *", updates.join(", "));
    params.push(id.to_string());

    // 执行更新
    let mut query = sqlx::query_as::<_, FertilizerMaterial>(&sql);
    for param in params.iter() {
        query = query.bind(param);
    }

    match query.fetch_one(&state.db_pool).await {
        Ok(material) => {
            let response = FertilizerResponse {
                success: true,
                message: "肥料/基质信息更新成功".to_string(),
                material: Some(material),
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to update fertilizer material: {}", e);
            let response = FertilizerResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                material: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 删除肥料与基质信息
pub async fn delete_fertilizer_material(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<FertilizerResponse>) {
    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM fertilizer_materials WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing fertilizer material: {}", e);
            let response = FertilizerResponse {
                success: false,
                message: format!("验证失败: {}", e),
                material: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = FertilizerResponse {
            success: false,
            message: format!("未找到ID为 {} 的肥料/基质信息", id),
            material: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 执行删除
    match sqlx::query("DELETE FROM fertilizer_materials WHERE id = ?")
        .bind(id)
        .execute(&state.db_pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                let response = FertilizerResponse {
                    success: true,
                    message: "肥料/基质信息删除成功".to_string(),
                    material: None,
                };
                (StatusCode::OK, Json(response))
            } else {
                // 这应该不会发生，因为我们已经检查过存在性
                let response = FertilizerResponse {
                    success: false,
                    message: "删除操作未影响任何记录".to_string(),
                    material: None,
                };
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete fertilizer material: {}", e);
            let response = FertilizerResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                material: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}