use crate::types::*;
use chrono::NaiveDate;
use std::collections::HashMap;

/// Build a complete usage summary from cached session data (no disk I/O).
pub fn build_usage_summary_from_cache(
    sessions_cache: &HashMap<String, HashMap<String, SessionEntry>>,
    agents: &[ResolvedAgent],
) -> UsageSummary {
    build_summary(agents, |agent| sessions_cache.get(&agent.id).cloned())
}

/// Build a complete usage summary across all agents.
/// Reads ~/.openclaw/agents/<id>/sessions/sessions.json (flat map format).
/// Cost is calculated from token counts using a built-in pricing table
/// because OpenClaw does not store cost in sessions.json.
#[allow(dead_code)]
pub fn build_usage_summary(agents: &[ResolvedAgent]) -> UsageSummary {
    build_summary(agents, |agent| {
        let sessions_json = agent.sessions_dir.join("sessions.json");
        std::fs::read_to_string(&sessions_json)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    })
}

/// Core aggregation logic — shared by both the cache-based and disk-based variants.
/// `get_sessions` is called once per agent to obtain its session map.
fn build_summary(
    agents: &[ResolvedAgent],
    get_sessions: impl Fn(&ResolvedAgent) -> Option<HashMap<String, SessionEntry>>,
) -> UsageSummary {
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

    let now = chrono::Utc::now();
    let week_ago = now - chrono::Duration::days(7);

    for agent in agents {
        let session_map = match get_sessions(agent) {
            Some(m) => m,
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

        for (_key, entry) in &session_map {
            let cost = calculate_cost(entry);

            total_cost += cost;
            agent_cost += cost;
            total_input += entry.input_tokens;
            total_output += entry.output_tokens;
            total_cache_read += entry.cache_read;
            agent_input += entry.input_tokens;
            agent_output += entry.output_tokens;

            // updatedAt is ms since epoch → convert to DateTime for bucketing
            if let Some(updated_ms) = entry.updated_at {
                let secs = (updated_ms / 1000) as i64;
                if let Some(dt) = chrono::DateTime::from_timestamp(secs, 0) {
                    let date_str = dt.format("%Y-%m-%d").to_string();

                    if date_str == today {
                        today_cost += cost;
                    }
                    if dt >= week_ago {
                        week_cost += cost;
                    }

                    let daily = daily_map.entry(date_str.clone()).or_insert(DailyCost {
                        date: date_str,
                        cost: 0.0,
                        input_tokens: 0,
                        output_tokens: 0,
                    });
                    daily.cost += cost;
                    daily.input_tokens += entry.input_tokens;
                    daily.output_tokens += entry.output_tokens;
                }
            }

            // Per-model breakdown — key on "provider/model" for display clarity
            let model_key =
                model_display_key(entry.model.as_deref(), entry.model_provider.as_deref());
            let mc = model_map.entry(model_key.clone()).or_insert(ModelCost {
                model: model_key,
                cost: 0.0,
                input_tokens: 0,
                output_tokens: 0,
                cache_read_tokens: 0,
                turn_count: 0,
            });
            mc.cost += cost;
            mc.input_tokens += entry.input_tokens;
            mc.output_tokens += entry.output_tokens;
            mc.cache_read_tokens += entry.cache_read;
            mc.turn_count += 1; // one session = one logical "turn" at this granularity
        }

        agent_costs.push(AgentCost {
            agent_id: agent.id.clone(),
            agent_name: agent.name.clone(),
            cost: agent_cost,
            input_tokens: agent_input,
            output_tokens: agent_output,
            session_count: session_map.len(),
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
        bloated_sessions: Vec::new(), // sessions.json has no size info; detect elsewhere
    }
}

/// Calculate USD cost from token counts using a built-in pricing table.
/// OpenClaw does not store cost in sessions.json — we derive it here.
fn calculate_cost(entry: &SessionEntry) -> f64 {
    let model = entry.model.as_deref().unwrap_or("");
    let provider = entry.model_provider.as_deref().unwrap_or("");
    let (input_per_m, output_per_m) = pricing(model, provider);
    (entry.input_tokens as f64 * input_per_m + entry.output_tokens as f64 * output_per_m)
        / 1_000_000.0
}

/// Per-million-token prices (input, output) in USD.
/// Matched on model name substring (case-insensitive) then provider.
fn pricing(model: &str, provider: &str) -> (f64, f64) {
    let m = model.to_ascii_lowercase();
    let p = provider.to_ascii_lowercase();

    if m.contains("gpt-5.3-codex") {
        (2.50, 10.0)
    } else if m.contains("deepseek-chat") || (p.contains("deepseek") && m.contains("chat")) {
        (0.14, 0.28)
    } else if m.contains("deepseek") {
        (0.14, 0.28)
    } else if m.contains("claude-opus") {
        (15.0, 75.0)
    } else if m.contains("claude-haiku") {
        (0.25, 1.25)
    } else {
        // Default: claude-sonnet-class pricing
        (3.0, 15.0)
    }
}

/// Build a display key combining provider and model for the model breakdown table.
fn model_display_key(model: Option<&str>, provider: Option<&str>) -> String {
    match (provider, model) {
        (Some(p), Some(m)) => format!("{p}/{m}"),
        (None, Some(m)) => m.to_string(),
        (Some(p), None) => p.to_string(),
        (None, None) => "unknown".to_string(),
    }
}
