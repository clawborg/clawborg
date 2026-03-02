use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::openclaw::{config, sessions};
use crate::types::*;

/// GET /api/sessions — List all sessions across all agents
pub async fn list_sessions(
    State(state): State<AppState>,
) -> Result<Json<Vec<SessionSummary>>, (StatusCode, Json<ApiError>)> {
    let cfg = config::read_config(&state.openclaw_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(format!("Failed to read config: {e}"))),
        )
    })?;

    let resolved = config::resolve_agents(&cfg, &state.openclaw_dir);
    let all_sessions = sessions::read_all_sessions(&resolved);

    Ok(Json(all_sessions))
}
