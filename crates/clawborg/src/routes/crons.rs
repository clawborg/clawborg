use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::openclaw::{config, cron};
use crate::types::*;

/// GET /api/crons — List cron jobs with status
pub async fn list_crons(
    State(state): State<AppState>,
) -> Result<Json<Vec<CronEntry>>, (StatusCode, Json<ApiError>)> {
    let cfg = config::read_config(&state.openclaw_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(format!("Failed to read config: {e}"))),
        )
    })?;

    let resolved = config::resolve_agents(&cfg, &state.openclaw_dir);
    let crons = cron::build_cron_list(&cfg, &resolved);

    Ok(Json(crons))
}
