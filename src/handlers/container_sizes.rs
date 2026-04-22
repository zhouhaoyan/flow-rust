use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::handlers::AppState;
use crate::models::{ContainerSize, CreateContainerSize};

#[derive(Debug, Deserialize)]
pub struct ContainerQuery {
    pub container_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ContainerListResponse {
    pub success: bool,
    pub message: String,
    pub containers: Vec<ContainerSize>,
}

#[derive(Debug, Serialize)]
pub struct ContainerResponse {
    pub success: bool,
    pub message: String,
    pub container: Option<ContainerSize>,
}

/// 获取种植容器尺寸列表
pub async fn list_container_sizes(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ContainerQuery>,
) -> (StatusCode, Json<ContainerListResponse>) {
    // Build query
    let mut sql = "SELECT * FROM container_sizes".to_string();
    let mut params: Vec<String> = Vec::new();

    if let Some(container_type) = &query.container_type {
        sql.push_str(" WHERE container_type LIKE ?");
        params.push(format!("%{}%", container_type));
    }

    sql.push_str(" ORDER BY container_type");

    // Execute query with dynamic binding
    let mut query = sqlx::query_as::<_, ContainerSize>(&sql);
    for param in params.iter() {
        query = query.bind(param);
    }

    match query.fetch_all(&state.db_pool).await {
        Ok(containers) => {
            let response = ContainerListResponse {
                success: true,
                message: format!("找到 {} 条容器记录", containers.len()),
                containers,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch container sizes: {}", e);
            let response = ContainerListResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                containers: Vec::new(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 创建新的种植容器尺寸记录
pub async fn create_container_size(
    State(state): State<Arc<AppState>>,
    Json(create_container): Json<CreateContainerSize>,
) -> (StatusCode, Json<ContainerResponse>) {
    // Insert the container size
    match sqlx::query_as::<_, ContainerSize>(
        "INSERT INTO container_sizes (container_type, dimensions, quantity, notes) VALUES (?, ?, ?, ?) RETURNING *"
    )
        .bind(&create_container.container_type)
        .bind(create_container.dimensions)
        .bind(create_container.quantity)
        .bind(create_container.notes)
        .fetch_one(&state.db_pool)
        .await
    {
        Ok(container) => {
            let response = ContainerResponse {
                success: true,
                message: "容器尺寸记录创建成功".to_string(),
                container: Some(container),
            };
            (StatusCode::CREATED, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to create container size: {}", e);
            let response = ContainerResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                container: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 获取单个种植容器尺寸记录
pub async fn get_container_size(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<ContainerResponse>) {
    match sqlx::query_as::<_, ContainerSize>("SELECT * FROM container_sizes WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(Some(container)) => {
            let response = ContainerResponse {
                success: true,
                message: "找到容器尺寸记录".to_string(),
                container: Some(container),
            };
            (StatusCode::OK, Json(response))
        }
        Ok(None) => {
            let response = ContainerResponse {
                success: false,
                message: format!("未找到ID为 {} 的容器尺寸记录", id),
                container: None,
            };
            (StatusCode::NOT_FOUND, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch container size: {}", e);
            let response = ContainerResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                container: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateContainerSize {
    pub container_type: Option<String>,
    pub dimensions: Option<String>,
    pub quantity: Option<i64>,
    pub notes: Option<String>,
}

/// 更新种植容器尺寸记录
pub async fn update_container_size(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Json(update_container): Json<UpdateContainerSize>,
) -> (StatusCode, Json<ContainerResponse>) {
    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM container_sizes WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing container size: {}", e);
            let response = ContainerResponse {
                success: false,
                message: format!("验证失败: {}", e),
                container: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = ContainerResponse {
            success: false,
            message: format!("未找到ID为 {} 的容器尺寸记录", id),
            container: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 构建动态更新SQL
    let mut updates = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(container_type) = &update_container.container_type {
        updates.push("container_type = ?");
        params.push(container_type.clone());
    }
    if let Some(dimensions) = &update_container.dimensions {
        updates.push("dimensions = ?");
        params.push(dimensions.clone());
    }
    if let Some(quantity) = &update_container.quantity {
        updates.push("quantity = ?");
        params.push(quantity.to_string());
    }
    if let Some(notes) = &update_container.notes {
        updates.push("notes = ?");
        params.push(notes.clone());
    }

    if updates.is_empty() {
        let response = ContainerResponse {
            success: false,
            message: "没有提供要更新的字段".to_string(),
            container: None,
        };
        return (StatusCode::BAD_REQUEST, Json(response));
    }

    // 构建完整SQL
    let sql = format!("UPDATE container_sizes SET {} WHERE id = ? RETURNING *", updates.join(", "));
    params.push(id.to_string());

    // 执行更新
    let mut query = sqlx::query_as::<_, ContainerSize>(&sql);
    for param in params.iter() {
        query = query.bind(param);
    }

    match query.fetch_one(&state.db_pool).await {
        Ok(container) => {
            let response = ContainerResponse {
                success: true,
                message: "容器尺寸记录更新成功".to_string(),
                container: Some(container),
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to update container size: {}", e);
            let response = ContainerResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                container: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 删除种植容器尺寸记录
pub async fn delete_container_size(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<ContainerResponse>) {
    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM container_sizes WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing container size: {}", e);
            let response = ContainerResponse {
                success: false,
                message: format!("验证失败: {}", e),
                container: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = ContainerResponse {
            success: false,
            message: format!("未找到ID为 {} 的容器尺寸记录", id),
            container: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 执行删除
    match sqlx::query("DELETE FROM container_sizes WHERE id = ?")
        .bind(id)
        .execute(&state.db_pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                let response = ContainerResponse {
                    success: true,
                    message: "容器尺寸记录删除成功".to_string(),
                    container: None,
                };
                (StatusCode::OK, Json(response))
            } else {
                // 这应该不会发生，因为我们已经检查过存在性
                let response = ContainerResponse {
                    success: false,
                    message: "删除操作未影响任何记录".to_string(),
                    container: None,
                };
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete container size: {}", e);
            let response = ContainerResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                container: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}