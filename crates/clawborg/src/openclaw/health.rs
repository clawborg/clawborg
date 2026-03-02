use crate::openclaw::{config, workspace};
use crate::types::*;
use std::path::Path;

/// Build a full health report across all agents
pub fn build_health_report(
    openclaw_dir: &Path,
) -> anyhow::Result<HealthReport> {
    let cfg = config::read_config(openclaw_dir)?;
    let agents = config::resolve_agents(&cfg, openclaw_dir);

    let mut agent_reports = Vec::new();

    for agent in &agents {
        let health = workspace::check_agent_health(agent);
        agent_reports.push(AgentHealthReport {
            agent_id: agent.id.clone(),
            status: health.status,
            issues: health.issues,
        });
    }

    let healthy_count = agent_reports
        .iter()
        .filter(|r| matches!(r.status, HealthStatus::Healthy))
        .count();

    let total_issues: usize = agent_reports.iter().map(|r| r.issues.len()).sum();

    Ok(HealthReport {
        total_agents: agent_reports.len(),
        healthy_agents: healthy_count,
        total_issues,
        agents: agent_reports,
    })
}

/// Pretty-print health report to stdout (for CLI `clawborg health`)
pub fn print_health_report(report: &HealthReport) {
    println!();
    println!("╔══════════════════════════════════════════╗");
    println!("║        ClawBorg Health Audit             ║");
    println!("╚══════════════════════════════════════════╝");
    println!();
    println!(
        "  Agents: {} total, {} healthy, {} issues",
        report.total_agents, report.healthy_agents, report.total_issues
    );
    println!();

    for agent_report in &report.agents {
        let icon = match agent_report.status {
            HealthStatus::Healthy => "🟢",
            HealthStatus::Warning => "🟡",
            HealthStatus::Critical => "🔴",
        };
        println!("  {} {}", icon, agent_report.agent_id);

        if agent_report.issues.is_empty() {
            println!("     All checks passed");
        } else {
            for issue in &agent_report.issues {
                let severity_icon = match issue.severity {
                    IssueSeverity::Critical => "  ✖",
                    IssueSeverity::Warning => "  ⚠",
                    IssueSeverity::Info => "  ℹ",
                };
                println!("   {} {}", severity_icon, issue.message);
            }
        }
        println!();
    }
}
