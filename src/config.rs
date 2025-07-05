//! # Cargo Dokita Configuration
//!
//! This module defines the configuration structures and logic for Cargo Dokita.
//!
//! ## Features
//! - Loads configuration from a TOML file (`.cargo-dokita.toml`) in the project root.
//! - Strictly validates configuration fields using Serde's `deny_unknown_fields`.
//! - Supports sections such as `[general]` and `[checks]` for extensible configuration.
//! - Allows enabling/disabling specific checks by code (e.g., `MD001`).
//! - Provides default values if no configuration file is found.
//! - Includes comprehensive tests for deserialization, error handling, and logic.
//!
//! ## Example `.cargo-dokita.toml`
//!
//! ```toml
//! [general]
//!
//! [checks]
//! enabled = { "MD001" = true, "MD002" = false }
//! ```
//!
//! ## Usage
//! Load configuration from the project root:
//! ```rust
//! let config = Config::load_from_project_root(project_root)?;
//! if config.is_check_enabled("MD001") {
//!     // Run check MD001
//! }
//! ```

//! 
//! # Config
//! 
//! This module defines the configuration structure for Cargo Dokita.
//! It uses TOML for configuration files and provides a way to load and validate the configuration.


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




#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.checks.enabled.is_empty());
    }

    #[test]
    fn test_config_deserialize_empty() {
        let toml_content = "";
        let config: Config = toml::from_str(toml_content).unwrap();
        assert!(config.checks.enabled.is_empty());
    }

    #[test]
    fn test_config_deserialize_minimal() {
        let toml_content = r#"
[general]

[checks]
"#;
        let config: Config = toml::from_str(toml_content).unwrap();
        assert!(config.checks.enabled.is_empty());
    }

    #[test]
    fn test_config_deserialize_with_checks() {
        let toml_content = r#"
[checks]
enabled = { "MD001" = true, "MD002" = false, "MD003" = true }
"#;
        let config: Config = toml::from_str(toml_content).unwrap();
        assert_eq!(config.checks.enabled.len(), 3);
        assert_eq!(config.checks.enabled.get("MD001"), Some(&true));
        assert_eq!(config.checks.enabled.get("MD002"), Some(&false));
        assert_eq!(config.checks.enabled.get("MD003"), Some(&true));
    }

    #[test]
    fn test_config_deserialize_unknown_field_error() {
        let toml_content = r#"
[general]
unknown_field = "value"
"#;
        let result: Result<Config, _> = toml::from_str(toml_content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown field"));
    }

    #[test]
    fn test_config_deserialize_unknown_checks_field_error() {
        let toml_content = r#"
[checks]
unknown_check_field = "value"
"#;
        let result: Result<Config, _> = toml::from_str(toml_content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown field"));
    }

    #[test]
    fn test_is_check_enabled_default_true() {
        let config = Config::default();
        assert!(config.is_check_enabled("MD001"));
        assert!(config.is_check_enabled("any_check"));
    }

    #[test]
    fn test_is_check_enabled_explicit_values() {
        let toml_content = r#"
[checks]
enabled = { "MD001" = true, "MD002" = false }
"#;
        let config: Config = toml::from_str(toml_content).unwrap();
        
        assert!(config.is_check_enabled("MD001"));
        assert!(!config.is_check_enabled("MD002"));
        // Should default to true for unspecified checks
        assert!(config.is_check_enabled("MD003"));
    }

    #[test]
    fn test_load_from_project_root_no_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let result = Config::load_from_project_root(temp_dir.path());
        
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.checks.enabled.is_empty());
    }

    #[test]
    fn test_load_from_project_root_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        
        let toml_content = r#"
[checks]
enabled = { "MD001" = true, "MD002" = false }
"#;
        fs::write(&config_path, toml_content).unwrap();
        
        let result = Config::load_from_project_root(temp_dir.path());
        assert!(result.is_ok());
        
        let config = result.unwrap();
        assert_eq!(config.checks.enabled.len(), 2);
        assert!(config.is_check_enabled("MD001"));
        assert!(!config.is_check_enabled("MD002"));
    }

    #[test]
    fn test_load_from_project_root_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        
        // Invalid TOML content
        let invalid_toml = r#"
[checks
enabled = { "MD001" = true }
"#;
        fs::write(&config_path, invalid_toml).unwrap();
        
        let result = Config::load_from_project_root(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse config file"));
    }

    #[test]
    fn test_load_from_project_root_invalid_config_structure() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        
        // Valid TOML but invalid config structure
        let invalid_config = r#"
[unknown_section]
field = "value"
"#;
        fs::write(&config_path, invalid_config).unwrap();
        
        let result = Config::load_from_project_root(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse config file"));
    }

    #[test]
    fn test_load_from_project_root_file_read_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
        
        // Create a directory instead of a file to simulate read error
        fs::create_dir(&config_path).unwrap();
        
        let result = Config::load_from_project_root(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read config file"));
    }

    #[test]
    fn test_config_file_name_constant() {
        assert_eq!(CONFIG_FILE_NAME, ".cargo-dokita.toml");
    }

    #[test]
    fn test_config_clone() {
        let toml_content = r#"
[checks]
enabled = { "MD001" = true, "MD002" = false }
"#;
        let config: Config = toml::from_str(toml_content).unwrap();
        let cloned_config = config.clone();
        
        assert_eq!(config.checks.enabled.len(), cloned_config.checks.enabled.len());
        assert_eq!(
            config.is_check_enabled("MD001"), 
            cloned_config.is_check_enabled("MD001")
        );
    }

    #[test]
    fn test_config_debug() {
        let config = Config::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("general"));
        assert!(debug_str.contains("checks"));
    }
}