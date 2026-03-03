use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

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
        .map(workspace::build_agent_summary)
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

#[derive(Deserialize)]
pub struct BrowseParams {
    /// Relative sub-path within the section (e.g. "memory" or "web-search")
    pub path: Option<String>,
    /// Section label — omit or "workspace" for main workspace, else a named dir label
    pub section: Option<String>,
}

/// GET /api/agents/:id/browse?path=subdir&section=label
/// Returns files and subdirectories at the given path within the specified section.
pub async fn browse_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<BrowseParams>,
) -> Result<Json<DirListing>, (StatusCode, Json<ApiError>)> {
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

    let subpath = params.path.as_deref().unwrap_or("");
    let section_label = params.section.as_deref().unwrap_or("workspace");

    // Determine the base directory for this section
    let (base_path, label) = if section_label == "workspace" || section_label.is_empty() {
        (agent.workspace_path.clone(), "workspace".to_string())
    } else {
        agent
            .named_dirs
            .iter()
            .find(|nd| nd.label == section_label)
            .map(|nd| (nd.path.clone(), nd.label.clone()))
            .ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ApiError::new(format!("Section not found: {section_label}"))),
                )
            })?
    };

    workspace::browse_workspace_dir(&base_path, subpath, &label).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiError::new(format!("{e}"))),
        )
    }).map(Json)
}
