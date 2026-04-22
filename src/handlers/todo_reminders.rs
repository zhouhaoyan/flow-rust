use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::handlers::AppState;
use crate::models::{TodoReminder, CreateTodoReminder, Priority};
use chrono::NaiveDate;

#[derive(Debug, Deserialize)]
pub struct TodoQuery {
    pub priority: Option<String>,
    pub completed: Option<bool>,
    pub due_date_before: Option<String>,
    pub due_date_after: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TodoListResponse {
    pub success: bool,
    pub message: String,
    pub todos: Vec<TodoReminder>,
}

#[derive(Debug, Serialize)]
pub struct TodoResponse {
    pub success: bool,
    pub message: String,
    pub todo: Option<TodoReminder>,
}

/// 获取待办与重要提醒列表
pub async fn list_todo_reminders(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TodoQuery>,
) -> (StatusCode, Json<TodoListResponse>) {
    // Build query
    let mut sql = "SELECT * FROM todo_reminders".to_string();
    let mut conditions = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(priority) = &query.priority {
        conditions.push("priority = ?");
        params.push(priority.clone());
    }

    if let Some(completed) = query.completed {
        conditions.push("completed = ?");
        params.push(completed.to_string());
    }

    if let Some(due_date_before) = &query.due_date_before {
        conditions.push("due_date <= ?");
        params.push(due_date_before.clone());
    }

    if let Some(due_date_after) = &query.due_date_after {
        conditions.push("due_date >= ?");
        params.push(due_date_after.clone());
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(" ORDER BY priority DESC, due_date ASC, created_at DESC");

    // Execute query with dynamic binding
    let mut query = sqlx::query_as::<_, TodoReminder>(&sql);
    for param in params.iter() {
        query = query.bind(param);
    }

    match query.fetch_all(&state.db_pool).await {
        Ok(todos) => {
            let response = TodoListResponse {
                success: true,
                message: format!("找到 {} 条待办提醒", todos.len()),
                todos,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch todo reminders: {}", e);
            let response = TodoListResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                todos: Vec::new(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 创建新的待办与重要提醒
pub async fn create_todo_reminder(
    State(state): State<Arc<AppState>>,
    Json(create_todo): Json<CreateTodoReminder>,
) -> (StatusCode, Json<TodoResponse>) {
    // Insert the todo reminder
    match sqlx::query_as::<_, TodoReminder>(
        "INSERT INTO todo_reminders (content, priority, due_date, notes) VALUES (?, ?, ?, ?) RETURNING *"
    )
        .bind(&create_todo.content)
        .bind(create_todo.priority)
        .bind(create_todo.due_date)
        .bind(create_todo.notes)
        .fetch_one(&state.db_pool)
        .await
    {
        Ok(todo) => {
            let response = TodoResponse {
                success: true,
                message: "待办提醒创建成功".to_string(),
                todo: Some(todo),
            };
            (StatusCode::CREATED, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to create todo reminder: {}", e);
            let response = TodoResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                todo: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 获取单个待办与重要提醒
pub async fn get_todo_reminder(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<TodoResponse>) {
    match sqlx::query_as::<_, TodoReminder>("SELECT * FROM todo_reminders WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(Some(todo)) => {
            let response = TodoResponse {
                success: true,
                message: "找到待办提醒".to_string(),
                todo: Some(todo),
            };
            (StatusCode::OK, Json(response))
        }
        Ok(None) => {
            let response = TodoResponse {
                success: false,
                message: format!("未找到ID为 {} 的待办提醒", id),
                todo: None,
            };
            (StatusCode::NOT_FOUND, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch todo reminder: {}", e);
            let response = TodoResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                todo: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 标记待办提醒为完成或未完成
pub async fn update_todo_status(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Json(status_update): Json<TodoStatusUpdate>,
) -> (StatusCode, Json<TodoResponse>) {
    let completed = status_update.completed;

    let sql = if completed {
        "UPDATE todo_reminders SET completed = TRUE, completed_at = CURRENT_TIMESTAMP WHERE id = ? RETURNING *"
    } else {
        "UPDATE todo_reminders SET completed = FALSE, completed_at = NULL WHERE id = ? RETURNING *"
    };

    match sqlx::query_as::<_, TodoReminder>(sql)
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(Some(todo)) => {
            let status = if completed { "完成" } else { "未完成" };
            let response = TodoResponse {
                success: true,
                message: format!("待办提醒已标记为{}", status),
                todo: Some(todo),
            };
            (StatusCode::OK, Json(response))
        }
        Ok(None) => {
            let response = TodoResponse {
                success: false,
                message: format!("未找到ID为 {} 的待办提醒", id),
                todo: None,
            };
            (StatusCode::NOT_FOUND, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to update todo status: {}", e);
            let response = TodoResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                todo: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TodoStatusUpdate {
    pub completed: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTodoReminder {
    pub content: Option<String>,
    pub priority: Option<Priority>,
    #[serde(default, deserialize_with = "crate::models::deserialize_optional_naive_date")]
    pub due_date: Option<NaiveDate>,
    pub notes: Option<String>,
}

/// 更新待办与重要提醒
pub async fn update_todo_reminder(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Json(update_todo): Json<UpdateTodoReminder>,
) -> (StatusCode, Json<TodoResponse>) {
    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM todo_reminders WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing todo reminder: {}", e);
            let response = TodoResponse {
                success: false,
                message: format!("验证失败: {}", e),
                todo: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = TodoResponse {
            success: false,
            message: format!("未找到ID为 {} 的待办提醒", id),
            todo: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 构建动态更新SQL
    let mut updates = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(content) = &update_todo.content {
        updates.push("content = ?");
        params.push(content.clone());
    }
    if let Some(priority) = &update_todo.priority {
        updates.push("priority = ?");
        // Priority enum to string
        params.push(format!("{:?}", priority));
    }
    if let Some(due_date) = &update_todo.due_date {
        updates.push("due_date = ?");
        params.push(due_date.to_string()); // NaiveDate to string
    }
    if let Some(notes) = &update_todo.notes {
        updates.push("notes = ?");
        params.push(notes.clone());
    }

    if updates.is_empty() {
        let response = TodoResponse {
            success: false,
            message: "没有提供要更新的字段".to_string(),
            todo: None,
        };
        return (StatusCode::BAD_REQUEST, Json(response));
    }

    // 构建完整SQL
    let sql = format!("UPDATE todo_reminders SET {} WHERE id = ? RETURNING *", updates.join(", "));
    params.push(id.to_string());

    // 执行更新
    let mut query = sqlx::query_as::<_, TodoReminder>(&sql);
    for param in params.iter() {
        query = query.bind(param);
    }

    match query.fetch_one(&state.db_pool).await {
        Ok(todo) => {
            let response = TodoResponse {
                success: true,
                message: "待办提醒更新成功".to_string(),
                todo: Some(todo),
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to update todo reminder: {}", e);
            let response = TodoResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                todo: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 删除待办与重要提醒
pub async fn delete_todo_reminder(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<TodoResponse>) {
    // 首先检查记录是否存在
    let existing: Option<(i64,)> = match sqlx::query_as("SELECT id FROM todo_reminders WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Failed to check existing todo reminder: {}", e);
            let response = TodoResponse {
                success: false,
                message: format!("验证失败: {}", e),
                todo: None,
            };
            return (StatusCode::BAD_REQUEST, Json(response));
        }
    };

    if existing.is_none() {
        let response = TodoResponse {
            success: false,
            message: format!("未找到ID为 {} 的待办提醒", id),
            todo: None,
        };
        return (StatusCode::NOT_FOUND, Json(response));
    }

    // 执行删除
    match sqlx::query("DELETE FROM todo_reminders WHERE id = ?")
        .bind(id)
        .execute(&state.db_pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                let response = TodoResponse {
                    success: true,
                    message: "待办提醒删除成功".to_string(),
                    todo: None,
                };
                (StatusCode::OK, Json(response))
            } else {
                // 这应该不会发生，因为我们已经检查过存在性
                let response = TodoResponse {
                    success: false,
                    message: "删除操作未影响任何记录".to_string(),
                    todo: None,
                };
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete todo reminder: {}", e);
            let response = TodoResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                todo: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}