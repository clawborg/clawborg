use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::openclaw::{config, workspace};
use crate::types::*;

#[derive(Deserialize)]
pub struct FileUpdateBody {
    pub content: String,
}

#[derive(Deserialize)]
pub struct FileParams {
    /// Optional section label — omit for workspace, else a named dir label
    pub section: Option<String>,
}

/// GET /api/agents/:id/files/*path?section=label — Read a file
/// Supports nested paths (e.g. "memory/2026-03-01.md") and named sections.
pub async fn get_file(
    State(state): State<AppState>,
    Path((id, subpath)): Path<(String, String)>,
    Query(params): Query<FileParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let cfg = config::read_config(&state.openclaw_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(format!("Config error: {e}"))),
        )
    })?;

    let resolved = config::resolve_agents(&cfg, &state.openclaw_dir);
    let agent = config::find_agent(&resolved, &id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiError::new(format!("Agent not found: {id}"))),
        )
    })?;

    let base_path = resolve_section_base(agent, params.section.as_deref()).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiError::new("Section not found".to_string())),
        )
    })?;

    let content = workspace::read_workspace_file(&base_path, &subpath).map_err(|e| {
        (StatusCode::NOT_FOUND, Json(ApiError::new(format!("{e}"))))
    })?;

    Ok(Json(serde_json::json!({
        "filename": subpath,
        "content": content,
    })))
}

/// PUT /api/agents/:id/files/*path — Update a workspace file (workspace only, .md files)
pub async fn update_file(
    State(state): State<AppState>,
    Path((id, subpath)): Path<(String, String)>,
    Json(body): Json<FileUpdateBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    if state.readonly {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiError::new(
                "Server is in read-only mode. Start without --readonly to enable writes.",
            )),
        ));
    }

    let cfg = config::read_config(&state.openclaw_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(format!("Config error: {e}"))),
        )
    })?;

    let resolved = config::resolve_agents(&cfg, &state.openclaw_dir);
    let agent = config::find_agent(&resolved, &id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiError::new(format!("Agent not found: {id}"))),
        )
    })?;

    workspace::write_workspace_file(&agent.workspace_path, &subpath, &body.content).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(ApiError::new(format!("{e}"))))
    })?;

    tracing::info!("📝 Updated {}/{}", agent.id, subpath);

    Ok(Json(serde_json::json!({
        "status": "ok",
        "filename": subpath,
        "agent_id": id,
    })))
}

/// Resolve the base path for a given section label.
/// None label or "workspace" → workspace_path. Named label → named_dir path.
fn resolve_section_base(agent: &ResolvedAgent, section: Option<&str>) -> Option<std::path::PathBuf> {
    match section {
        None | Some("workspace") | Some("") => Some(agent.workspace_path.clone()),
        Some(label) => agent
            .named_dirs
            .iter()
            .find(|nd| nd.label == label)
            .map(|nd| nd.path.clone()),
    }
}
