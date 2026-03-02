use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::openclaw::{config, workspace};
use crate::types::*;

#[derive(Deserialize)]
pub struct FileUpdateBody {
    pub content: String,
}

/// GET /api/agents/:id/files/:filename — Read a workspace file
pub async fn get_file(
    State(state): State<AppState>,
    Path((id, filename)): Path<(String, String)>,
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

    let content =
        workspace::read_workspace_file(&agent.workspace_path, &filename).map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(ApiError::new(format!("{e}"))),
            )
        })?;

    Ok(Json(serde_json::json!({
        "filename": filename,
        "content": content,
    })))
}

/// PUT /api/agents/:id/files/:filename — Update a workspace file
pub async fn update_file(
    State(state): State<AppState>,
    Path((id, filename)): Path<(String, String)>,
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

    workspace::write_workspace_file(&agent.workspace_path, &filename, &body.content)
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiError::new(format!("{e}"))),
            )
        })?;

    tracing::info!("📝 Updated {}/{}", agent.id, filename);

    Ok(Json(serde_json::json!({
        "status": "ok",
        "filename": filename,
        "agent_id": id,
    })))
}
