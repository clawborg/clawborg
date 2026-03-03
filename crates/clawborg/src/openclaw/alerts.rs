use crate::types::*;
use chrono::Utc;

/// Generate smart alerts from all available data.
/// `alerts_config` is the optional `alerts` block from openclaw.json.
pub fn generate_alerts(
    usage: &UsageSummary,
    crons: &[CronEntry],
    health_report: &HealthReport,
    critical_threshold: f64,
    warning_threshold: f64,
) -> Vec<Alert> {
    let mut alerts = Vec::new();
    let now = Utc::now().to_rfc3339();

    // ── Cost alerts ──
    if usage.today_cost >= critical_threshold {
        alerts.push(Alert {
            id: "cost-critical".to_string(),
            severity: AlertSeverity::Critical,
            category: "cost".to_string(),
            title: "High daily spend".to_string(),
            message: format!(
                "Today's cost is ${:.2}, exceeding ${:.0} threshold",
                usage.today_cost, critical_threshold
            ),
            timestamp: now.clone(),
        });
    } else if usage.today_cost >= warning_threshold {
        alerts.push(Alert {
            id: "cost-warning".to_string(),
            severity: AlertSeverity::Warning,
            category: "cost".to_string(),
            title: "Elevated daily spend".to_string(),
            message: format!(
                "Today's cost is ${:.2}, approaching ${:.0} threshold",
                usage.today_cost, critical_threshold
            ),
            timestamp: now.clone(),
        });
    }

    // ── Bloated sessions ──
    for session in &usage.bloated_sessions {
        alerts.push(Alert {
            id: format!("bloat-{}-{}", session.agent_id, session.session_key),
            severity: AlertSeverity::Warning,
            category: "session".to_string(),
            title: "Bloated session file".to_string(),
            message: format!(
                "Session {} ({}) is {} — consider resetting with /new",
                session.session_key, session.agent_id, session.size_display
            ),
            timestamp: now.clone(),
        });
    }

    // ── Cron alerts ──
    for cron in crons {
        if matches!(cron.status, CronStatus::Overdue) {
            alerts.push(Alert {
                id: format!("cron-overdue-{}", cron.agent),
                severity: AlertSeverity::Warning,
                category: "cron".to_string(),
                title: "Cron job overdue".to_string(),
                message: format!(
                    "{} cron for agent '{}' appears overdue ({})",
                    cron.task, cron.agent, cron.schedule_display
                ),
                timestamp: now.clone(),
            });
        }
    }

    // ── Health alerts ──
    for agent_health in &health_report.agents {
        if matches!(agent_health.status, HealthStatus::Critical) {
            let critical_count = agent_health
                .issues
                .iter()
                .filter(|i| matches!(i.severity, IssueSeverity::Critical))
                .count();
            alerts.push(Alert {
                id: format!("health-critical-{}", agent_health.agent_id),
                severity: AlertSeverity::Critical,
                category: "health".to_string(),
                title: format!("Agent '{}' has critical issues", agent_health.agent_id),
                message: format!(
                    "{} critical issue(s) detected. Check Health page for details.",
                    critical_count
                ),
                timestamp: now.clone(),
            });
        }
    }

    // ── Missing bootstrap ──
    for agent_health in &health_report.agents {
        for issue in &agent_health.issues {
            if matches!(issue.severity, IssueSeverity::Critical)
                && issue.message.contains("instruction files")
            {
                alerts.push(Alert {
                    id: format!("bootstrap-missing-{}", agent_health.agent_id),
                    severity: AlertSeverity::Critical,
                    category: "config".to_string(),
                    title: "Missing bootstrap files".to_string(),
                    message: format!(
                        "Agent '{}': {}",
                        agent_health.agent_id, issue.message
                    ),
                    timestamp: now.clone(),
                });
            }
        }
    }

    // Sort: critical first, then warning
    alerts.sort_by(|a, b| {
        let order = |s: &AlertSeverity| match s {
            AlertSeverity::Critical => 0,
            AlertSeverity::Warning => 1,
        };
        order(&a.severity).cmp(&order(&b.severity))
    });

    alerts
}
