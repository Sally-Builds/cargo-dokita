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
        if config.is_check_enabled("MD001") {
            if package.description.is_none() || package.description.as_deref() == Some("") {
                findings.push(Finding::new(
                    "MD001",
                    "Missing 'description' in [package] section of Cargo.toml.".to_string(),
                    Severity::Warning,
                    Some("Cargo.toml".to_string()),
                ));
            }
        }
        
        if config.is_check_enabled("MD002") {
             if package.license.is_none() || package.license.as_deref() == Some("") {
                findings.push(Finding::new(
                "MD002",
                "Missing 'license' (or 'license-file') in [package] section of Cargo.toml.".to_string(),
                Severity::Warning,
                Some("Cargo.toml".to_string()),
                ));
            }
        }

        if config.is_check_enabled("MD003") {
            if package.repository.is_none() || package.repository.as_deref() == Some("") {
                findings.push(Finding::new(
                    "MD003",
                    "Missing 'repository' in [package] section of Cargo.toml.".to_string(),
                    Severity::Note, // Less critical than license/description for local projects
                    Some("Cargo.toml".to_string()),
                ));
            }
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
                    if readme_value.as_str().is_some() {

                    }else if readme_value.as_bool() == Some(false) {

                    }else {
                        findings.push(Finding::new(
                            "MD004",
                            format!("The 'readme' field in Cargo.toml has an unexpected value ( '{}' ). Expected a file path string (e.g., \"README.md\") or `false`.", readme_value.to_string()),
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

pub fn check_rust_edition(manifest: &CargoManifest, config: &Config) -> Vec<Finding> {
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


// #[cfg(test)]
// mod tests {
//     use super::*;


//     #[test]
//     fn parse_valid_cargo_manifest_from_file() {
//         // create temp dir and toml file
//         let temp_dir = std::env::temp_dir();
//         let  temp_file = temp_dir.join("Cargo-test.toml");

//         let toml_content = r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2024"
//         "#;

//         //write to toml file
//         fs::write(&temp_file, toml_content).unwrap();

//         let _cleanup = scopeguard::guard(&temp_file, |path| {
//             let _ = fs::remove_file(path);
//         });
        
//         let result = CargoManifest::parse(temp_file.as_path());
//         assert!(result.is_ok());

//         let cargo_manifest = result.unwrap();
//         let package = cargo_manifest.package.as_ref().unwrap();

//         assert_eq!(package.name, "cargo-dokita");
//         assert_eq!(package.version, "0.1.0");
//         assert_eq!(package.edition, Some("2024".to_string()));
//     }

//     #[test]
//     fn parse_invalid_cargo_manifest_from_file() {
//         let temp_dir = std::env::temp_dir();
//         let  temp_file = temp_dir.join("Cargo-test.toml");

//         let toml_content = r#"
//             package
//             name: "cargo-dokita"
//             version = "0.1.0"
//             edition = "2024"
//         "#;

//         //write to toml file
//         fs::write(&temp_file, toml_content).unwrap();

//         let _cleanup = scopeguard::guard(&temp_file, |path| {
//             let _ = fs::remove_file(path);
//         });

//         let result = CargoManifest::parse(temp_file.as_path());
//         let err_msg = result.unwrap_err();
//         assert!(err_msg.contains("Failed to parse Cargo.toml at"));
//     }

//     #[test]
//     fn parse_cargo_manifest_from_invalid_path() {
//         let temp_dir = std::env::temp_dir();
//         let  temp_file = temp_dir.join("");

//         let _cleanup = scopeguard::guard(&temp_file, |path| {
//             let _ = fs::remove_file(path);
//         });

//         let result = CargoManifest::parse(temp_file.as_path());
//         let err_msg = result.unwrap_err();
//         assert!(err_msg.contains("Failed to read Cargo.toml at"));
//     }

//     #[test]
//     fn complete_metadata_returns_no_findings() {
//         let manifest  = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2024"
//             description =  "A project that checks your code structure"
//             license = "MIT"
//             repository = "https://github.com/Sally-Builds/Rustify"
//             readme = "README.md"
//         "#).expect("Failed to pass TOML");


//         let findings = check_missing_metadata(&manifest);

//         assert!(findings.is_empty(), "Expected no findings for complete metadata, but got: {:#?}", findings);
//     }

//     #[test]
//     fn missing_metadata_description_returns_md001_finding() {
//         let manifest  = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2024"
//             license = "MIT"
//             repository = "https://github.com/Sally-Builds/Rustify"
//             readme = "README.md"
//         "#).expect("Failed to pass TOML");

//         let findings = check_missing_metadata(&manifest);

//         assert!(findings.len() == 1);
//         assert!(findings[0].message.contains("Missing 'description' in [package]"));
//         assert_eq!(findings[0].code, "MD001");
//         assert_eq!(findings[0].severity, Severity::Warning);
//     }

//     #[test]
//     fn missing_metadata_license_returns_md002_findings() {
//         let manifest  = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             description =  "A projects that checks your code structure"
//             repository = "http://github.com/Sally-Builds/Rustify"
//             edition = "2024"
//             readme = "README.md"
//         "#).expect("Failed to pass TOML");

//         let findings = check_missing_metadata(&manifest);

//         assert!(findings.len() == 1);
//         assert!(findings[0].message.contains("Missing 'license' (or 'license-file') in [package]"));
//         assert_eq!(findings[0].code, "MD002");
//         assert_eq!(findings[0].severity, Severity::Warning);
//     }

//     #[test]
//     fn missing_metadata_repository_returns_md0003_findings() {
//         let manifest  = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             description =  "A projects that checks your code structure"
//             edition = "2024"
//             license = "MIT"
//             readme = "README.md"
//         "#).expect("Failed to pass TOML");

//         let findings = check_missing_metadata(&manifest);

//         assert!(findings.len() == 1);
//         assert!(findings[0].message.contains("Missing 'repository' in [package]"));
//         assert_eq!(findings[0].code, "MD003");
//         assert_eq!(findings[0].severity, Severity::Note);
//     }

//     #[test]
//     fn missing_metadata_readme_returns_md0004_findings() {
//         let manifest  = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             description =  "A projects that checks your code structure"
//             repository = "http://github.com/Sally-Builds/Rustify"
//             edition = "2024"
//             license = "MIT"

//         "#).expect("Failed to pass TOML");

//         let findings = check_missing_metadata(&manifest);

//         assert!(findings.len() == 1);
//         assert!(findings[0].message.contains("Missing 'readme' field in [package] section of Cargo.toml"));
//         assert_eq!(findings[0].code, "MD004");
//         assert_eq!(findings[0].severity, Severity::Note);
//     }

//     #[test]
//     fn specific_version_returns_no_findings() {
//         let manifest = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2021"
            
//             [dependencies]
//             serde = { version = "1.0.219", features = ["derive"] }
//             toml = "0.8.22"
//         "#).expect("Failed to parse TOML");

//         let findings = check_dependency_versions(&manifest);

//         assert!(findings.is_empty(), "Expected no findings, but got: {:?}", findings);
//     }

//     #[test]
//     fn wildcard_version_returns_dp001_finding() {
//         let manifest = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2021"
            
//             [dependencies]
//             serde = { version = "*", features = ["derive"] }
//             toml = "0.8.22"
//         "#).expect("Failed to parse TOML");

//         let findings = check_dependency_versions(&manifest);

//         assert_eq!(findings.len(), 1, "Expected exactly 1 finding, got {}", findings.len());
        
//         let finding = &findings[0];
//         assert_eq!(finding.code, "DP001");
//         assert_eq!(finding.severity, Severity::Warning);
//         assert!(finding.message.contains("Wildcard version \"*\" used for runtime dependency 'serde'"));
//         assert_eq!(finding.file_path, Some("Cargo.toml".to_string()));
//     }

//     #[test]
//     fn multiple_wildcard_dependencies_returns_multiple_findings() {
//         let manifest = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2021"
            
//             [dependencies]
//             serde = "*"
//             tokio = "*"
//             reqwest = "0.11.0"
//         "#).expect("Failed to parse TOML");

//         let findings = check_dependency_versions(&manifest);

//         assert_eq!(findings.len(), 2);
//         assert!(findings.iter().all(|f| f.code == "DP001"));
//         assert!(findings.iter().any(|f| f.message.contains("'serde'")));
//         assert!(findings.iter().any(|f| f.message.contains("'tokio'")));
//     }

//     #[test]
//     fn wildcard_in_dev_dependencies_returns_dp001_finding() {
//         let manifest = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2021"
            
//             [dependencies]
//             serde = "1.0.0"
            
//             [dev-dependencies]
//             criterion = "*"
//         "#).expect("Failed to parse TOML");

//         let findings = check_dependency_versions(&manifest);

//         assert_eq!(findings.len(), 1);
//         let finding = &findings[0];
//         assert_eq!(finding.code, "DP001");
//         assert!(finding.message.contains("dev dependency 'criterion'"));
//     }

//     #[test]
//     fn wildcard_in_build_dependencies_returns_dp001_finding() {
//         let manifest = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2021"
            
//             [build-dependencies]
//             cc = "*"
//         "#).expect("Failed to parse TOML");

//         let findings = check_dependency_versions(&manifest);

//         assert_eq!(findings.len(), 1);
//         let finding = &findings[0];
//         assert_eq!(finding.code, "DP001");
//         assert!(finding.message.contains("build dependency 'cc'"));
//     }

//     #[test]
//     fn mixed_dependency_types_with_wildcards() {
//         let manifest = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2021"
            
//             [dependencies]
//             serde = "*"
            
//             [dev-dependencies]
//             criterion = "*"
            
//             [build-dependencies]
//             cc = "1.0.0"
//         "#).expect("Failed to parse TOML");

//         let findings = check_dependency_versions(&manifest);

//         assert_eq!(findings.len(), 2);
//         assert!(findings.iter().any(|f| f.message.contains("runtime dependency 'serde'")));
//         assert!(findings.iter().any(|f| f.message.contains("dev dependency 'criterion'")));
//     }

//     #[test]
//     fn empty_dependencies_returns_no_findings() {
//         let manifest = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2021"
//         "#).expect("Failed to parse TOML");

//         let findings = check_dependency_versions(&manifest);

//         assert!(findings.is_empty());
//     }


//     #[test]
//     fn latest_stable_edition_returns_no_findings() {
//         let manifest = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2024"
//         "#).expect("Failed to parse TOML");

//         let findings = check_rust_edition(&manifest);

//         assert!(findings.is_empty(), "Expected no findings but got {:?}", findings);
//     }

//     #[test]
//     fn rust_2021_edition_returns_ed001_finding() {
//         let manifest = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2021"
//         "#).expect("Failed to parse TOML");

//         let findings = check_rust_edition(&manifest);

//         assert_eq!(findings.len(), 1, "Expected exactly 1 finding");
        
//         let finding = &findings[0];
//         assert_eq!(finding.code, "ED001");
//         assert_eq!(finding.severity, Severity::Note);
//         assert!(finding.message.contains("Project uses Rust edition '2021', consider updating to '2024'"));
//         assert_eq!(finding.file_path, Some("Cargo.toml".to_string()));
//     }

//     #[test]
//     fn rust_2018_edition_returns_ed001_finding() {
//         let manifest = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2018"
//         "#).expect("Failed to parse TOML");

//         let findings = check_rust_edition(&manifest);

//         assert_eq!(findings.len(), 1);
        
//         let finding = &findings[0];
//         assert_eq!(finding.code, "ED001");
//         assert_eq!(finding.severity, Severity::Note);
//         assert!(finding.message.contains("Project uses Rust edition '2018', consider updating to '2024'"));
//     }

//     #[test]
//     fn rust_2015_edition_returns_ed001_finding() {
//         let manifest = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "2015"
//         "#).expect("Failed to parse TOML");

//         let findings = check_rust_edition(&manifest);

//         assert_eq!(findings.len(), 1);
        
//         let finding = &findings[0];
//         assert_eq!(finding.code, "ED001");
//         assert_eq!(finding.severity, Severity::Note);
//         assert!(finding.message.contains("Project uses Rust edition '2015', consider updating to '2024'"));
//     }

//     #[test]
//     fn no_edition_specified_returns_ed002_finding() {
//         let manifest = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//         "#).expect("Failed to parse TOML");

//         let findings = check_rust_edition(&manifest);

//         assert_eq!(findings.len(), 1, "Expected exactly 1 finding");
        
//         let finding = &findings[0];
//         assert_eq!(finding.code, "ED002");
//         assert_eq!(finding.severity, Severity::Note);
//         assert!(finding.message.contains("Project does not specify a Rust edition (implicitly 2015)"));
//         assert!(finding.message.contains("consider specifying and updating to '2024'"));
//         assert_eq!(finding.file_path, Some("Cargo.toml".to_string()));
//     }

//     #[test]
//     fn no_package_section_returns_no_findings() {
//         let manifest = toml::from_str(r#"
//             [workspace]
//             members = ["crate1", "crate2"]
//         "#).expect("Failed to parse TOML");

//         let findings = check_rust_edition(&manifest);

//         assert!(findings.is_empty(), "Expected no findings for workspace-only manifest");
//     }

//     #[test]
//     fn empty_manifest_returns_no_findings() {
//         let manifest = toml::from_str("").expect("Failed to parse empty TOML");

//         let findings = check_rust_edition(&manifest);

//         assert!(findings.is_empty(), "Expected no findings for empty manifest");
//     }

//     // Edge case: what happens with invalid edition values?
//     #[test]
//     fn invalid_edition_returns_ed001_finding() {
//         let manifest = toml::from_str(r#"
//             [package]
//             name = "cargo-dokita"
//             version = "0.1.0"
//             edition = "invalid"
//         "#).expect("Failed to parse TOML");

//         let findings = check_rust_edition(&manifest);

//         assert_eq!(findings.len(), 1);
        
//         let finding = &findings[0];
//         assert_eq!(finding.code, "ED001");
//         assert!(finding.message.contains("Project uses Rust edition 'invalid'"));
//     }
// }


