use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::openclaw::{config, workspace};
use crate::types::*;

/// GET /api/agents — List all agents
pub async fn list_agents(
    State(state): State<AppState>,
) -> Result<Json<Vec<AgentSummary>>, (StatusCode, Json<ApiError>)> {
    let cfg = config::read_config(&state.openclaw_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(format!("Failed to read config: {e}"))),
        )
    })?;

    let resolved = config::resolve_agents(&cfg, &state.openclaw_dir);

    let agents: Vec<AgentSummary> = resolved
        .iter()
        .map(|agent| workspace::build_agent_summary(agent))
        .collect();

    Ok(Json(agents))
}

/// GET /api/agents/:id — Get agent detail
pub async fn get_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<AgentDetail>, (StatusCode, Json<ApiError>)> {
    let cfg = config::read_config(&state.openclaw_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(format!("Failed to read config: {e}"))),
        )
    })?;

    let resolved = config::resolve_agents(&cfg, &state.openclaw_dir);

    let agent = config::find_agent(&resolved, &id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiError::new(format!("Agent not found: {id}"))),
        )
    })?;

    let detail = workspace::build_agent_detail(agent);
    Ok(Json(detail))
}
