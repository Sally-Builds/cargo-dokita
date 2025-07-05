//! Manifest parsing and metadata checks for Cargo.toml files.
//!
//! This module provides data structures and functions for parsing Cargo manifests (Cargo.toml),
//! extracting package and dependency information, and performing various metadata checks.
//!
//! # Features
//!
//! - Defines `Package`, `Dependency`, and `CargoManifest` structs for deserializing Cargo.toml.
//! - Supports both simple and detailed dependency specifications.
//! - Provides `CargoManifest::parse` for loading and parsing a manifest from disk.
//! - Implements checks for missing or incomplete package metadata (description, license, repository, readme, etc.).
//! - Checks for wildcard dependency versions and outdated or missing Rust edition fields.
//! - Includes comprehensive unit tests for manifest parsing and metadata validation.
//!
//! This module is intended for use in tools that lint, audit, or analyze Rust project manifests.
//! 

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::fs;

use crate::{config::Config, diagnostics::{Finding, Severity}};

#[derive(Deserialize,Serialize, Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub edition: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    // pub readme: Option<String>,
    pub readme: Option<toml::Value>,
    pub repository: Option<String>,
}


#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Dependency {
    Version(String),
    Detailed(DetailedDependency),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DetailedDependency {
    pub version: Option<String>,
    pub path: Option<String>,
    pub features: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CargoManifest {
    pub package: Option<Package>, // Package section is optional (e.g. in a workspace virtual manifest)
    pub dependencies: Option<HashMap<String, Dependency>>,
    #[serde(rename = "dev-dependencies")]
    pub dev_dependencies: Option<HashMap<String, Dependency>>,
    #[serde(rename = "build-dependencies")]
    pub build_dependencies: Option<HashMap<String, Dependency>>,
    // You can add `workspace`, `lib`, `bin` sections here if needed later
}

impl CargoManifest {
    pub fn parse(path_to_cargo_toml: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path_to_cargo_toml)
            .map_err(|e| format!("Failed to read Cargo.toml at {:?}: {}", path_to_cargo_toml, e))?;

        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse Cargo.toml at {:?}: {}", path_to_cargo_toml, e))
    }
}


pub fn check_missing_metadata(manifest: &CargoManifest, config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();
    if let Some(package) = &manifest.package {
        if config.is_check_enabled("MD001") && (package.description.is_none() || package.description.as_deref() == Some("")) {
                findings.push(Finding::new(
                    "MD001",
                    "Missing 'description' in [package] section of Cargo.toml.".to_string(),
                    Severity::Warning,
                    Some("Cargo.toml".to_string()),
                ));
        }
        
        if config.is_check_enabled("MD002") && (package.license.is_none() || package.license.as_deref() == Some("")) {
                findings.push(Finding::new(
                "MD002",
                "Missing 'license' (or 'license-file') in [package] section of Cargo.toml.".to_string(),
                Severity::Warning,
                Some("Cargo.toml".to_string()),
                ));
        }

        if config.is_check_enabled("MD003") && (package.repository.is_none() || package.repository.as_deref() == Some("")) {
                findings.push(Finding::new(
                    "MD003",
                    "Missing 'repository' in [package] section of Cargo.toml.".to_string(),
                    Severity::Note, // Less critical than license/description for local projects
                    Some("Cargo.toml".to_string()),
                ));
        }
        
        if config.is_check_enabled("MD004") {
            match &package.readme {
                None => {
                    findings.push(Finding::new(
                        "MD004",
                        "Missing 'readme' field in [package] section of Cargo.toml. Consider adding `readme = \"README.md\"` or `readme = false`.".to_string(),
                        Severity::Note,
                        Some("Cargo.toml".to_string()),
                    ));
                },
                Some(readme_value) => {
                    if readme_value.as_str().is_some()  || readme_value.as_bool() == Some(false){

                    }else {
                        findings.push(Finding::new(
                            "MD004",
                            format!("The 'readme' field in Cargo.toml has an unexpected value ( '{}' ). Expected a file path string (e.g., \"README.md\") or `false`.", readme_value),
                            Severity::Warning, // This is more than a note, it's likely a misconfiguration.
                            Some("Cargo.toml".to_string()),
                        ));
                    }

                }
            }
        }
        // Add more checks: authors, keywords, categories if desired
    } else {
        findings.push(Finding::new(
            "MD005",
            "Missing section [package]".to_string(),
            Severity::Error,
            Some("Cargo.toml".to_string()),
        ));
    }
    findings
}

pub fn check_dependency_versions(manifest: &CargoManifest, config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut check_deps = |deps: &Option<HashMap<String, Dependency>>, dep_type: &str| {
        if let Some(dependencies) = deps {
            for (name, dep) in dependencies {
                let version_str = match dep {
                    Dependency::Version(s) => Some(s.as_str()),
                    Dependency::Detailed(d) => d.version.as_deref(),
                };

                if version_str == Some("*") {
                    findings.push(Finding::new(
                        "DP001",
                        format!(
                            "Wildcard version \"*\" used for {} dependency '{}'. Specify a version range.",
                            dep_type, name
                        ),
                        Severity::Warning,
                        Some("Cargo.toml".to_string()),
                    ));
                }
                // Could add more checks: overly broad versions like ">0.1", etc.
            }
        }
    };

    if config.is_check_enabled("DP001") {
        check_deps(&manifest.dependencies, "runtime");
        check_deps(&manifest.dev_dependencies, "dev");
        check_deps(&manifest.build_dependencies, "build");
    }
    

    findings
}

pub fn check_rust_edition(manifest: &CargoManifest) -> Vec<Finding> {
    let mut findings = Vec::new();
    const LATEST_STABLE_EDITION: &str = "2024"; // Update this as new editions are released

    if let Some(package) = &manifest.package {
        match &package.edition {
            Some(edition) if edition != LATEST_STABLE_EDITION => {
                findings.push(Finding::new(
                    "ED001",
                    format!(
                        "Project uses Rust edition '{}', consider updating to '{}'.",
                        edition, LATEST_STABLE_EDITION
                    ),
                    Severity::Note,
                    Some("Cargo.toml".to_string()),
                ));
            }
            None => { // Editions before 2018 were implicit (2015)
                findings.push(Finding::new(
                    "ED002",
                    format!(
                        "Project does not specify a Rust edition (implicitly 2015), consider specifying and updating to '{}'.",
                         LATEST_STABLE_EDITION
                    ),
                    Severity::Note,
                    Some("Cargo.toml".to_string()),
                ));
            }
            _ => {} // Edition is latest or not applicable
        }
    }
    findings
}





#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::collections::HashMap;
    use tempfile::TempDir;
    use crate::config::{Config, GeneralConfig, ChecksConfig};
    use crate::diagnostics::{Severity};

    // Helper function to create a temporary Cargo.toml file
    fn create_temp_cargo_toml(content: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_toml_path, content).unwrap();
        (temp_dir, cargo_toml_path)
    }

    // Helper function to create a mock config with all checks enabled
    fn mock_config_all_enabled() -> Config {
        let mut enabled = HashMap::new();
        enabled.insert("MD001".to_string(), true);
        enabled.insert("MD002".to_string(), true);
        enabled.insert("MD003".to_string(), true);
        enabled.insert("MD004".to_string(), true);
        
        Config {
            general: GeneralConfig::default(),
            checks: ChecksConfig { enabled },
        }
    }

    // Helper function to create a mock config with specific checks enabled
    fn mock_config_with_checks(checks: &[&str]) -> Config {
        let mut enabled = HashMap::new();
        for check in checks {
            enabled.insert(check.to_string(), true);
        }
        
        Config {
            general: GeneralConfig::default(),
            checks: ChecksConfig { enabled },
        }
    }

    // Helper function to create a mock config with specific checks disabled
    fn _mock_config_with_disabled_checks(disabled_checks: &[&str]) -> Config {
        let mut enabled = HashMap::new();
        // Enable all checks by default
        enabled.insert("MD001".to_string(), true);
        enabled.insert("MD002".to_string(), true);
        enabled.insert("MD003".to_string(), true);
        enabled.insert("MD004".to_string(), true);
        
        // Disable specific checks
        for check in disabled_checks {
            enabled.insert(check.to_string(), false);
        }
        
        Config {
            general: GeneralConfig::default(),
            checks: ChecksConfig { enabled },
        }
    }

    #[test]
    fn test_parse_complete_cargo_toml() {
        let content = r#"
[package]
name = "test-package"
version = "0.1.0"
edition = "2021"
description = "A test package"
license = "MIT"
readme = "README.md"
repository = "https://github.com/user/repo"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
tempfile = "3.0"

[build-dependencies]
cc = "1.0"
"#;

        let (_temp_dir, path) = create_temp_cargo_toml(content);
        let manifest = CargoManifest::parse(&path).unwrap();

        assert!(manifest.package.is_some());
        let package = manifest.package.unwrap();
        assert_eq!(package.name, "test-package");
        assert_eq!(package.version, "0.1.0");
        assert_eq!(package.edition, Some("2021".to_string()));
        assert_eq!(package.description, Some("A test package".to_string()));
        assert_eq!(package.license, Some("MIT".to_string()));
        assert_eq!(package.repository, Some("https://github.com/user/repo".to_string()));

        assert!(manifest.dependencies.is_some());
        assert!(manifest.dev_dependencies.is_some());
        assert!(manifest.build_dependencies.is_some());
    }

    #[test]
    fn test_parse_minimal_cargo_toml() {
        let content = r#"
[package]
name = "minimal-package"
version = "0.1.0"
"#;

        let (_temp_dir, path) = create_temp_cargo_toml(content);
        let manifest = CargoManifest::parse(&path).unwrap();

        assert!(manifest.package.is_some());
        let package = manifest.package.unwrap();
        assert_eq!(package.name, "minimal-package");
        assert_eq!(package.version, "0.1.0");
        assert_eq!(package.edition, None);
        assert_eq!(package.description, None);
        assert_eq!(package.license, None);
        assert_eq!(package.repository, None);
    }

    #[test]
    fn test_parse_workspace_cargo_toml() {
        let content = r#"
[workspace]
members = ["crate1", "crate2"]

[dependencies]
shared-dep = "1.0"
"#;

        let (_temp_dir, path) = create_temp_cargo_toml(content);
        let manifest = CargoManifest::parse(&path).unwrap();

        assert!(manifest.package.is_none());
        assert!(manifest.dependencies.is_some());
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let result = CargoManifest::parse(Path::new("nonexistent/Cargo.toml"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read Cargo.toml"));
    }

    #[test]
    fn test_parse_invalid_toml() {
        let content = "invalid toml content [[[";
        let (_temp_dir, path) = create_temp_cargo_toml(content);
        let result = CargoManifest::parse(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse Cargo.toml"));
    }

    #[test]
    fn test_dependency_parsing() {
        let content = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
simple = "1.0"
detailed = { version = "2.0", features = ["feature1"] }
path_dep = { path = "../local" }
"#;

        let (_temp_dir, path) = create_temp_cargo_toml(content);
        let manifest = CargoManifest::parse(&path).unwrap();

        let deps = manifest.dependencies.unwrap();
        
        match deps.get("simple").unwrap() {
            Dependency::Version(v) => assert_eq!(v, "1.0"),
            _ => panic!("Expected version dependency"),
        }

        match deps.get("detailed").unwrap() {
            Dependency::Detailed(d) => {
                assert_eq!(d.version, Some("2.0".to_string()));
                assert_eq!(d.features, Some(vec!["feature1".to_string()]));
            },
            _ => panic!("Expected detailed dependency"),
        }

        match deps.get("path_dep").unwrap() {
            Dependency::Detailed(d) => {
                assert_eq!(d.path, Some("../local".to_string()));
                assert_eq!(d.version, None);
            },
            _ => panic!("Expected detailed dependency"),
        }
    }

    #[test]
    fn test_check_missing_metadata_all_present() {
        let manifest = CargoManifest {
            package: Some(Package {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                edition: Some("2021".to_string()),
                description: Some("Test description".to_string()),
                license: Some("MIT".to_string()),
                readme: Some(toml::Value::String("README.md".to_string())),
                repository: Some("https://github.com/user/repo".to_string()),
            }),
            dependencies: None,
            dev_dependencies: None,
            build_dependencies: None,
        };

        let config = mock_config_all_enabled();
        let findings = check_missing_metadata(&manifest, &config);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_check_missing_description() {
        let manifest = CargoManifest {
            package: Some(Package {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                edition: None,
                description: None,
                license: Some("MIT".to_string()),
                readme: Some(toml::Value::String("README.md".to_string())),
                repository: Some("https://github.com/user/repo".to_string()),
            }),
            dependencies: None,
            dev_dependencies: None,
            build_dependencies: None,
        };

        let config = mock_config_with_checks(&["MD001"]);
        let findings = check_missing_metadata(&manifest, &config);
        
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "MD001");
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].message.contains("Missing 'description'"));
    }

    #[test]
    fn test_check_empty_description() {
        let manifest = CargoManifest {
            package: Some(Package {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                edition: None,
                description: Some("".to_string()),
                license: Some("MIT".to_string()),
                readme: Some(toml::Value::String("README.md".to_string())),
                repository: Some("https://github.com/user/repo".to_string()),
            }),
            dependencies: None,
            dev_dependencies: None,
            build_dependencies: None,
        };

        let config = mock_config_with_checks(&["MD001"]);
        let findings = check_missing_metadata(&manifest, &config);
        
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "MD001");
    }

    #[test]
    fn test_check_missing_license() {
        let manifest = CargoManifest {
            package: Some(Package {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                edition: None,
                description: Some("Test description".to_string()),
                license: None,
                readme: Some(toml::Value::String("README.md".to_string())),
                repository: Some("https://github.com/user/repo".to_string()),
            }),
            dependencies: None,
            dev_dependencies: None,
            build_dependencies: None,
        };

        let config = mock_config_with_checks(&["MD002"]);
        let findings = check_missing_metadata(&manifest, &config);
        
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "MD002");
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].message.contains("Missing 'license'"));
    }

    #[test]
    fn test_check_missing_repository() {
        let manifest = CargoManifest {
            package: Some(Package {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                edition: None,
                description: Some("Test description".to_string()),
                license: Some("MIT".to_string()),
                readme: Some(toml::Value::String("README.md".to_string())),
                repository: None,
            }),
            dependencies: None,
            dev_dependencies: None,
            build_dependencies: None,
        };

        let config = mock_config_with_checks(&["MD003"]);
        let findings = check_missing_metadata(&manifest, &config);
        
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "MD003");
        assert_eq!(findings[0].severity, Severity::Note);
        assert!(findings[0].message.contains("Missing 'repository'"));
    }

    #[test]
    fn test_check_missing_readme() {
        let manifest = CargoManifest {
            package: Some(Package {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                edition: None,
                description: Some("Test description".to_string()),
                license: Some("MIT".to_string()),
                readme: None,
                repository: Some("https://github.com/user/repo".to_string()),
            }),
            dependencies: None,
            dev_dependencies: None,
            build_dependencies: None,
        };

        let config = mock_config_with_checks(&["MD004"]);
        let findings = check_missing_metadata(&manifest, &config);
        
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "MD004");
        assert_eq!(findings[0].severity, Severity::Note);
        assert!(findings[0].message.contains("Missing 'readme' field"));
    }

    #[test]
    fn test_check_readme_false() {
        let manifest = CargoManifest {
            package: Some(Package {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                edition: None,
                description: Some("Test description".to_string()),
                license: Some("MIT".to_string()),
                readme: Some(toml::Value::Boolean(false)),
                repository: Some("https://github.com/user/repo".to_string()),
            }),
            dependencies: None,
            dev_dependencies: None,
            build_dependencies: None,
        };

        let config = mock_config_with_checks(&["MD004"]);
        let findings = check_missing_metadata(&manifest, &config);
        
        assert!(findings.is_empty());
    }

    #[test]
    fn test_check_readme_invalid_value() {
        let manifest = CargoManifest {
            package: Some(Package {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                edition: None,
                description: Some("Test description".to_string()),
                license: Some("MIT".to_string()),
                readme: Some(toml::Value::Integer(123)),
                repository: Some("https://github.com/user/repo".to_string()),
            }),
            dependencies: None,
            dev_dependencies: None,
            build_dependencies: None,
        };

        let config = mock_config_with_checks(&["MD004"]);
        let findings = check_missing_metadata(&manifest, &config);
        
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "MD004");
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].message.contains("unexpected value"));
    }

    #[test]
    fn test_check_missing_package_section() {
        let manifest = CargoManifest {
            package: None,
            dependencies: Some(HashMap::new()),
            dev_dependencies: None,
            build_dependencies: None,
        };

        let config = mock_config_all_enabled();
        let findings = check_missing_metadata(&manifest, &config);
        
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "MD005");
        assert_eq!(findings[0].severity, Severity::Error);
        assert!(findings[0].message.contains("Missing section [package]"));
    }

    #[test]
    fn test_check_multiple_missing_fields() {
        let manifest = CargoManifest {
            package: Some(Package {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                edition: None,
                description: None,
                license: None,
                readme: None,
                repository: None,
            }),
            dependencies: None,
            dev_dependencies: None,
            build_dependencies: None,
        };

        let config = mock_config_all_enabled();
        let findings = check_missing_metadata(&manifest, &config);
        
        assert_eq!(findings.len(), 4); // MD001, MD002, MD003, MD004
        
        let codes: Vec<&str> = findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"MD001"));
        assert!(codes.contains(&"MD002"));
        assert!(codes.contains(&"MD003"));
        assert!(codes.contains(&"MD004"));
    }

    #[test]
    fn test_check_disabled_checks() {
    let manifest = CargoManifest {
        package: Some(Package {
            name: "test".to_string(),
            version: "0.1.0".to_string(),
            edition: None,
            description: None,
            license: None,
            readme: None,
            repository: None,
        }),
        dependencies: None,
        dev_dependencies: None,
        build_dependencies: None,
    };

    // Create config with only MD001 enabled, others explicitly disabled
    let mut enabled = HashMap::new();
    enabled.insert("MD001".to_string(), true);
    enabled.insert("MD002".to_string(), false);
    enabled.insert("MD003".to_string(), false);
    enabled.insert("MD004".to_string(), false);
    
    let config = Config {
        general: GeneralConfig::default(),
        checks: ChecksConfig { enabled },
    };
    
    let findings = check_missing_metadata(&manifest, &config);
    
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, "MD001");
}


    #[test]
    fn test_readme_field_variants() {
        // Test string value
        let mut manifest = CargoManifest {
            package: Some(Package {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                edition: None,
                description: Some("Test".to_string()),
                license: Some("MIT".to_string()),
                readme: Some(toml::Value::String("README.md".to_string())),
                repository: Some("https://github.com/user/repo".to_string()),
            }),
            dependencies: None,
            dev_dependencies: None,
            build_dependencies: None,
        };

        let config = mock_config_with_checks(&["MD004"]);
        let findings = check_missing_metadata(&manifest, &config);
        assert!(findings.is_empty());

        // Test boolean true value (should be treated as invalid)
        manifest.package.as_mut().unwrap().readme = Some(toml::Value::Boolean(true));
        let findings = check_missing_metadata(&manifest, &config);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "MD004");
        assert_eq!(findings[0].severity, Severity::Warning);
    }
}