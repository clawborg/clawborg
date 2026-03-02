use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::openclaw::health;
use crate::types::*;

/// GET /api/health — Full health audit
pub async fn health_audit(
    State(state): State<AppState>,
) -> Result<Json<HealthReport>, (StatusCode, Json<ApiError>)> {
    let report = health::build_health_report(&state.openclaw_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(format!("Health audit failed: {e}"))),
        )
    })?;

    Ok(Json(report))
}
