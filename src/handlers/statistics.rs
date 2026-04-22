use axum::{
    extract::{Query, State, Path},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::handlers::AppState;
use crate::models::GerminationStats;

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub batch: Option<String>, // "第一批" or "第二批"
    pub plant_short_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GerminationStatsListResponse {
    pub success: bool,
    pub message: String,
    pub stats: Vec<GerminationStats>,
}

#[derive(Debug, Serialize)]
pub struct GerminationStatsResponse {
    pub success: bool,
    pub message: String,
    pub stat: Option<GerminationStats>,
}

/// 获取出芽率与活苗率统计
pub async fn list_germination_stats(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StatsQuery>,
) -> (StatusCode, Json<GerminationStatsListResponse>) {
    // Build query
    let mut sql = "SELECT * FROM germination_stats".to_string();
    let mut conditions = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(batch) = &query.batch {
        conditions.push("batch = ?");
        params.push(batch.clone());
    }

    if let Some(plant_short_name) = &query.plant_short_name {
        conditions.push("plant_short_name = ?");
        params.push(plant_short_name.clone());
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    sql.push_str(" ORDER BY batch, plant_short_name");

    // Execute query with dynamic binding
    let mut query = sqlx::query_as::<_, GerminationStats>(&sql);
    for param in params.iter() {
        query = query.bind(param);
    }

    match query.fetch_all(&state.db_pool).await {
        Ok(stats) => {
            let response = GerminationStatsListResponse {
                success: true,
                message: format!("找到 {} 条统计记录", stats.len()),
                stats,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch germination stats: {}", e);
            let response = GerminationStatsListResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                stats: Vec::new(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 计算并更新出芽率与活苗率统计
pub async fn calculate_germination_stats(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<GerminationStatsListResponse>) {
    // This would be a complex calculation from growth logs
    // For now, we'll just return existing stats
    // In a full implementation, this would:
    // 1. Query growth logs for each plant and batch
    // 2. Calculate seeds_sown, seeds_germinated, seeds_dead
    // 3. Update germination_stats table

    match sqlx::query_as::<_, GerminationStats>("SELECT * FROM germination_stats ORDER BY batch, plant_short_name")
        .fetch_all(&state.db_pool)
        .await
    {
        Ok(stats) => {
            let response = GerminationStatsListResponse {
                success: true,
                message: format!("找到 {} 条统计记录", stats.len()),
                stats,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch germination stats: {}", e);
            let response = GerminationStatsListResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                stats: Vec::new(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

/// 获取特定品种的统计信息
pub async fn get_plant_germination_stats(
    Path((batch, plant_short_name)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<GerminationStatsResponse>) {
    match sqlx::query_as::<_, GerminationStats>(
        "SELECT * FROM germination_stats WHERE batch = ? AND plant_short_name = ?"
    )
        .bind(batch)
        .bind(plant_short_name)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(Some(stat)) => {
            let response = GerminationStatsResponse {
                success: true,
                message: "找到统计信息".to_string(),
                stat: Some(stat),
            };
            (StatusCode::OK, Json(response))
        }
        Ok(None) => {
            let response = GerminationStatsResponse {
                success: false,
                message: "未找到该品种的统计信息".to_string(),
                stat: None,
            };
            (StatusCode::NOT_FOUND, Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to fetch germination stats: {}", e);
            let response = GerminationStatsResponse {
                success: false,
                message: format!("数据库错误: {}", e),
                stat: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}