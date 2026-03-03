use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::openclaw::{alerts, config, cron, health, usage};
use crate::types::*;

/// GET /api/alerts — Get smart alerts
pub async fn get_alerts(
    State(state): State<AppState>,
) -> Result<Json<Vec<Alert>>, (StatusCode, Json<ApiError>)> {
    let cfg = config::read_config(&state.openclaw_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(format!("Failed to read config: {e}"))),
        )
    })?;

    let resolved = config::resolve_agents(&cfg, &state.openclaw_dir);

    // Read from cache for fast access
    let (usage_summary, cron_list) = {
        let cache = state.cache.read().await;
        let usage_summary = usage::build_usage_summary_from_cache(&cache.sessions, &resolved);
        let cron_list = cron::build_cron_list_from_jobs(&cache.cron_jobs);
        (usage_summary, cron_list)
    };

    let health_report = health::build_health_report(&state.openclaw_dir).unwrap_or(HealthReport {
        total_agents: 0,
        healthy_agents: 0,
        total_issues: 0,
        agents: Vec::new(),
    });

    let critical_threshold = state.clawborg_config.alerts.critical_threshold();
    let warning_threshold = state.clawborg_config.alerts.warning_threshold();
    let alert_list = alerts::generate_alerts(
        &usage_summary,
        &cron_list,
        &health_report,
        critical_threshold,
        warning_threshold,
    );

    Ok(Json(alert_list))
}
