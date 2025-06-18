use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const CONFIG_FILE_NAME: &str = ".cargo-dokita.toml";

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)] // Be strict about unknown config keys
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub checks: ChecksConfig,
    // You could add more sections like 'thresholds', 'ignores', etc.
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct GeneralConfig {
    // Example: Minimum severity to report (Error, Warning, Note)
    // pub report_severity_level: Option<String>, // Could map to Severity enum
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct ChecksConfig {
    // Key: Check code (e.g., "MD001"), Value: enabled (true/false)
    #[serde(default)]
    pub enabled: HashMap<String, bool>,
    // Example: specific config for a check
    // pub max_todo_comments: Option<usize>,
}

impl Config {
    pub fn load_from_project_root(project_root: &Path) -> Result<Self, String> {
        let config_path = project_root.join(CONFIG_FILE_NAME);
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config file {:?}: {}", config_path, e))?;
            toml::from_str(&content)
                .map_err(|e| format!("Failed to parse config file {:?}: {}", config_path, e))
        } else {
            // Return default config if no file found
            Ok(Config::default())
        }
    }

    /// Check if a specific check code is enabled.
    /// Defaults to true if not specified in the config.
    pub fn is_check_enabled(&self, check_code: &str) -> bool {
        self.checks.enabled.get(check_code).copied().unwrap_or(true)
    }
}

