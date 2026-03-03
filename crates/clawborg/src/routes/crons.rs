use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::openclaw::cron;
use crate::types::*;

/// GET /api/crons — List cron jobs with status
pub async fn list_crons(
    State(state): State<AppState>,
) -> Result<Json<Vec<CronEntry>>, (StatusCode, Json<ApiError>)> {
    let cache = state.cache.read().await;
    let crons = cron::build_cron_list_from_jobs(&cache.cron_jobs);

    Ok(Json(crons))
}
