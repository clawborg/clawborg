use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::openclaw::{config, usage};
use crate::types::*;

/// GET /api/usage — Get cost and token usage summary
pub async fn get_usage(
    State(state): State<AppState>,
) -> Result<Json<UsageSummary>, (StatusCode, Json<ApiError>)> {
    let cfg = config::read_config(&state.openclaw_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(format!("Failed to read config: {e}"))),
        )
    })?;

    let resolved = config::resolve_agents(&cfg, &state.openclaw_dir);
    let summary = usage::build_usage_summary(&resolved);

    Ok(Json(summary))
}
