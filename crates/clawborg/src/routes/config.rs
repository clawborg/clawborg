use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::openclaw::config;
use crate::types::*;

/// GET /api/config — Redacted OpenClaw config
pub async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let redacted = config::read_config_redacted(&state.openclaw_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(format!("Failed to read config: {e}"))),
        )
    })?;

    Ok(Json(redacted))
}
