use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::openclaw::config;
use crate::types::*;

/// GET /api/agents/:id/tasks — List tasks for an agent
/// Returns task counts and file listing per folder.
/// Only available if the agent's workspace has a tasks/ directory.
pub async fn list_tasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
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

    let tasks_dir = agent.workspace_path.join("tasks");
    if !tasks_dir.exists() {
        return Ok(Json(serde_json::json!({
            "agent_id": id,
            "has_task_queue": false,
            "message": "This agent does not use a task queue"
        })));
    }

    let list_folder = |folder: &str| -> Vec<serde_json::Value> {
        let dir = tasks_dir.join(folder);
        if !dir.exists() {
            return Vec::new();
        }
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let fname = entry.file_name().to_string_lossy().to_string();
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let modified = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());
                files.push(serde_json::json!({
                    "name": fname,
                    "size": size,
                    "modified": modified,
                }));
            }
        }
        files
    };

    Ok(Json(serde_json::json!({
        "agent_id": id,
        "has_task_queue": true,
        "pending": list_folder("pending"),
        "approved": list_folder("approved"),
        "done": list_folder("done"),
    })))
}
