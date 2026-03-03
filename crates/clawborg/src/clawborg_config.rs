use serde::Deserialize;
use std::path::PathBuf;

const DEFAULT_CRITICAL: f64 = 20.0;
const DEFAULT_WARNING: f64 = 5.0;

/// ClawBorg's own configuration, loaded from ~/.clawborg/config.toml.
/// This is separate from openclaw.json — ClawBorg never writes non-standard
/// fields into OpenClaw's config.
///
/// Example ~/.clawborg/config.toml:
///   [alerts]
///   dailySpendThreshold = 50.0
///   dailySpendWarning = 10.0
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ClawBorgConfig {
    #[serde(default)]
    pub alerts: AlertsConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AlertsConfig {
    /// Daily cost that triggers a critical alert (USD). Default: $20.
    pub daily_spend_threshold: Option<f64>,
    /// Daily cost that triggers a warning alert (USD). Default: $5.
    pub daily_spend_warning: Option<f64>,
}

impl AlertsConfig {
    pub fn critical_threshold(&self) -> f64 {
        self.daily_spend_threshold.unwrap_or(DEFAULT_CRITICAL)
    }
    pub fn warning_threshold(&self) -> f64 {
        self.daily_spend_warning.unwrap_or(DEFAULT_WARNING)
    }
}

/// Load ClawBorg config from ~/.clawborg/config.toml.
/// Logs exactly once at startup: found or not found.
/// Returns defaults silently if the file is absent.
pub fn load() -> ClawBorgConfig {
    let config_path = config_path();

    let Some(path) = config_path else {
        tracing::info!("No ClawBorg config found, using defaults");
        return ClawBorgConfig::default();
    };

    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<ClawBorgConfig>(&content) {
            Ok(cfg) => {
                tracing::info!("Loaded ClawBorg config from {}", path.display());
                cfg
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse {}: {e} — using defaults",
                    path.display()
                );
                ClawBorgConfig::default()
            }
        },
        Err(_) => {
            tracing::info!("No ClawBorg config found, using defaults");
            ClawBorgConfig::default()
        }
    }
}

fn config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".clawborg").join("config.toml"))
}
