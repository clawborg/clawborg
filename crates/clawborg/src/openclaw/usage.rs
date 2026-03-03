use crate::types::*;
use chrono::NaiveDate;
use std::collections::HashMap;

/// Build a complete usage summary across all agents by reading sessions.json
pub fn build_usage_summary(agents: &[ResolvedAgent]) -> UsageSummary {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let today_date = NaiveDate::parse_from_str(&today, "%Y-%m-%d").ok();

    let mut total_cost: f64 = 0.0;
    let mut today_cost: f64 = 0.0;
    let mut week_cost: f64 = 0.0;
    let mut total_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut total_cache_read: u64 = 0;

    let mut model_map: HashMap<String, ModelCost> = HashMap::new();
    let mut agent_costs: Vec<AgentCost> = Vec::new();
    let mut daily_map: HashMap<String, DailyCost> = HashMap::new();
    let mut bloated: Vec<BloatedSession> = Vec::new();

    let now = chrono::Utc::now();
    let week_ago = now - chrono::Duration::days(7);

    for agent in agents {
        let sessions_json = agent.sessions_dir.join("sessions.json");

        let store = match std::fs::read_to_string(&sessions_json)
            .ok()
            .and_then(|s| serde_json::from_str::<SessionStore>(&s).ok())
        {
            Some(s) => s,
            None => {
                agent_costs.push(AgentCost {
                    agent_id: agent.id.clone(),
                    agent_name: agent.name.clone(),
                    cost: 0.0,
                    input_tokens: 0,
                    output_tokens: 0,
                    session_count: 0,
                });
                continue;
            }
        };

        let mut agent_cost: f64 = 0.0;
        let mut agent_input: u64 = 0;
        let mut agent_output: u64 = 0;

        for entry in &store.sessions {
            total_cost += entry.cost;
            agent_cost += entry.cost;
            total_input += entry.input_tokens;
            total_output += entry.output_tokens;
            total_cache_read += entry.cache_read_tokens;
            agent_input += entry.input_tokens;
            agent_output += entry.output_tokens;

            // Parse last_active for date-based bucketing
            let last_active_dt = entry
                .last_active
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc));

            if let Some(dt) = last_active_dt {
                let date_str = dt.format("%Y-%m-%d").to_string();

                // Today's cost
                if date_str == today {
                    today_cost += entry.cost;
                }

                // Week cost
                if dt >= week_ago {
                    week_cost += entry.cost;
                }

                // Daily trend
                let daily = daily_map.entry(date_str.clone()).or_insert(DailyCost {
                    date: date_str,
                    cost: 0.0,
                    input_tokens: 0,
                    output_tokens: 0,
                });
                daily.cost += entry.cost;
                daily.input_tokens += entry.input_tokens;
                daily.output_tokens += entry.output_tokens;
            }

            // Per-model breakdown
            if let Some(ref model) = entry.model {
                let mc = model_map.entry(model.clone()).or_insert(ModelCost {
                    model: model.clone(),
                    cost: 0.0,
                    input_tokens: 0,
                    output_tokens: 0,
                    cache_read_tokens: 0,
                    turn_count: 0,
                });
                mc.cost += entry.cost;
                mc.input_tokens += entry.input_tokens;
                mc.output_tokens += entry.output_tokens;
                mc.cache_read_tokens += entry.cache_read_tokens;
                mc.turn_count += entry.turn_count;
            }

            // Bloated sessions (>500 KB)
            if entry.size_bytes > 500_000 {
                bloated.push(BloatedSession {
                    agent_id: agent.id.clone(),
                    session_key: entry.key.clone(),
                    size_bytes: entry.size_bytes,
                    size_display: format_bytes(entry.size_bytes),
                });
            }
        }

        agent_costs.push(AgentCost {
            agent_id: agent.id.clone(),
            agent_name: agent.name.clone(),
            cost: agent_cost,
            input_tokens: agent_input,
            output_tokens: agent_output,
            session_count: store.sessions.len(),
        });
    }

    // Sort models by cost descending
    let mut by_model: Vec<ModelCost> = model_map.into_values().collect();
    by_model.sort_by(|a, b| b.cost.partial_cmp(&a.cost).unwrap_or(std::cmp::Ordering::Equal));

    // Sort agents by cost descending
    agent_costs.sort_by(|a, b| b.cost.partial_cmp(&a.cost).unwrap_or(std::cmp::Ordering::Equal));

    // Sort daily trend by date, keep last 30 days
    let mut daily_trend: Vec<DailyCost> = daily_map.into_values().collect();
    daily_trend.sort_by(|a, b| a.date.cmp(&b.date));

    if let Some(cutoff_date) =
        today_date.and_then(|d| d.checked_sub_signed(chrono::Duration::days(30)))
    {
        let cutoff = cutoff_date.format("%Y-%m-%d").to_string();
        daily_trend.retain(|d| d.date >= cutoff);
    }

    // Sort bloated by size descending
    bloated.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    UsageSummary {
        total_cost,
        today_cost,
        week_cost,
        total_input_tokens: total_input,
        total_output_tokens: total_output,
        total_cache_read_tokens: total_cache_read,
        by_model,
        by_agent: agent_costs,
        daily_trend,
        bloated_sessions: bloated,
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
