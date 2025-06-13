use std::{path::Path, process::Command};

use cargo_metadata::{semver::Version, Metadata, MetadataCommand};
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::{crates_io_api, diagnostics::{Finding, Severity}};

pub fn get_project_metadata(manifest_path: &Path) -> Result<Metadata, cargo_metadata::Error> {
    MetadataCommand::new().manifest_path(manifest_path).exec()
}

pub fn check_outdated_dependencies(metadata: &Metadata, http_client: &Client) -> Vec<Finding> {
// pub fn check_outdated_dependencies(metadata: &Metadata, client: &Client) -> Vec<Finding> {
    let mut findings = Vec::new();

    for package_id in &metadata.workspace_members {
        let package = &metadata[package_id];

        if let Some(resolve) = &metadata.resolve {
            let package_node = resolve.nodes.iter().find(|n| n.id == *package_id);

            if let Some(node) = package_node {
                
                for dep_node_id in &node.deps {
                    let dep_package_info = &metadata[&dep_node_id.pkg]; // Info about the dependency crate
                    let resolved_version_str = dep_package_info.version.to_string();

                      if dep_package_info.source.is_none() || !dep_package_info.source.as_ref().unwrap().is_crates_io() {
                        // println!("Skipping non-crates.io dependency: {}", dep_package_info.name);
                        continue;
                    }
                }
            }
        }

        for dep in &package.dependencies {
            if dep.source.is_none() || !dep.source.as_ref().unwrap().is_crates_io() {
                continue;
            }

            let dep_name = &dep.name;

            let package_node_in_resolve = metadata.resolve.as_ref()
                .and_then(|r| r.nodes.iter().find(|n| n.id == *package_id));

            if let Some(p_node) = package_node_in_resolve {
                // Find the specific dependency instance for this package
                if let Some(resolved_dep_link) = p_node.deps.iter().find(|d| d.name.to_string() == *dep_name) {
                    let resolved_dep_package = &metadata[&resolved_dep_link.pkg];
                    let current_version_str = resolved_dep_package.version.to_string();

                    match crates_io_api::get_latest_versions_from_crates_io(&dep_name, http_client) {
                        Ok(latest_version_str) => {
                            let current_ver = Version::parse(&current_version_str);
                            let latest_ver = Version::parse(&latest_version_str);

                            if let (Ok(cur), Ok(latest)) = (current_ver, latest_ver) {
                                if cur < latest {
                                    findings.push(Finding::new(
                                        "DP002", // Outdated Dependency
                                        format!(
                                            "Direct dependency '{}' is outdated. Current: {}, Latest: {}",
                                            dep_name, cur, latest
                                        ),
                                        Severity::Note, // Or Warning, depending on preference
                                        Some("Cargo.toml".to_string()), // Or Cargo.lock
                                    ));
                                }
                            } else {
                                // Failed to parse versions, maybe log this
                                eprintln!("Warning: Could not parse versions for {}: current '{}', latest '{}'", dep_name, current_version_str, latest_version_str);
                            }
                        },
                        Err(e)  => {
                            eprintln!("Warning: Could not fetch latest version for {}: {}", dep_name, e);
                            // Optionally create a finding for API fetch failures
                             findings.push(Finding::new(
                                "API001",
                                format!("Failed to fetch latest version for dependency '{}': {}", dep_name, e),
                                Severity::Warning, // This is an issue with cargo-doctor itself or network
                                None,
                            ));
                        }
                    }
                }
            }

        }
    }

    findings
}

pub fn check_vulnerability(project_path: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();

    let output_result = Command::new("cargo")
        .arg("audit")
        .arg("--json") // Request JSON output for easier parsing
        .arg("--quiet") // Suppress non-JSON output from cargo-audit itself
        .current_dir(project_path) // Run in the context of the target project
        .output();

    match output_result {
        Ok(output) => {
            if !output.status.success() {

                if output.stdout.is_empty() && !output.stderr.is_empty() {
                    let stderr = String::from_utf8_lossy(&output.stderr);

                    findings.push(Finding::new(
                        "AUD001", // Audit Tool Error
                        format!("cargo-audit execution failed: {}", stderr.lines().next().unwrap_or("Unknown error")),
                        Severity::Warning, // This is a problem with the tool or setup
                        Some(project_path.join("Cargo.lock").to_string_lossy().into_owned())
                    ));

                    return findings;
                }
            }

            #[derive(Deserialize, Debug)]
            struct AuditReport {
                vulnerabilities: VulnerabilitiesList,
                // warnings: HashMap<String, Vec<AuditWarning>>, // If interested in warnings
            }

            #[derive(Deserialize, Debug)]
            struct VulnerabilitiesList {
                list: Vec<VulnerabilityEntry>,
            }

            #[derive(Deserialize, Debug)]
            struct VulnerabilityEntry {
                advisory: Advisory,
                package: AuditPackage,
                versions: VersionInfo,
            }

            #[derive(Deserialize, Debug)]
            struct Advisory {
                id: String,
                title: String,
                // url: String,
                // severity: String, // e.g., "high", "critical"
            }

            #[derive(Deserialize, Debug)]
            struct AuditPackage {
                name: String,
                // version: String, // The specific vulnerable version found
            }

            #[derive(Deserialize, Debug)]
            struct VersionInfo {
                patched: Vec<String>, // Patched versions
                // unaffected: Vec<String>,
            }


            match serde_json::from_slice::<AuditReport>(&output.stdout) {
                Ok(report) => {
                    if !report.vulnerabilities.list.is_empty() {
                        for vuln in report.vulnerabilities.list {
                            findings.push(Finding::new(
                                "SEC001", // Security Vulnerability
                                format!(
                                    "Vulnerability found in '{}': {} (ID: {}). Patched in: {:?}.",
                                    vuln.package.name,
                                    vuln.advisory.title,
                                    vuln.advisory.id,
                                    vuln.versions.patched.join(", ")
                                ),
                                Severity::Error, // Vulnerabilities are usually errors
                                Some(project_path.join("Cargo.lock").to_string_lossy().into_owned()),
                            ));
                        }
                    }else if !output.status.success() && !output.stdout.is_empty() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        findings.push(Finding::new(
                            "AUD002",
                            format!("cargo-audit indicated an issue but no vulnerabilities found in JSON: {}", stderr),
                            Severity::Warning,
                            Some(project_path.join("Cargo.lock").to_string_lossy().into_owned()),
                        ));
                    }

                },
                Err(e) => {
                    if !output.status.success() && !output.stdout.is_empty() {
                         let stdout_preview = String::from_utf8_lossy(&output.stdout);
                        eprintln!("Failed to parse cargo-audit JSON output: {}. Output: {}", e, stdout_preview.chars().take(200).collect::<String>());
                        findings.push(Finding::new(
                            "AUD003", // Audit Parse Error
                            format!("Failed to parse cargo-audit JSON output: {}", e),
                            Severity::Warning,
                            Some(project_path.join("Cargo.lock").to_string_lossy().into_owned()),
                        ));
                    }
                }
            } 
        },
        Err(e) => {
            // This means `cargo audit` command itself could not be run (e.g., not installed)
            findings.push(Finding::new(
                "AUD004", // Audit Not Found or Execution Error
                format!("Failed to execute 'cargo audit'. Is it installed and in PATH? Error: {}", e),
                Severity::Warning, // Can't perform check, so it's a warning for the user
                None,
            ));
        }
    }

    findings
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs, time::{SystemTime, UNIX_EPOCH}};

    // Helper function to create a unique temporary directory
    fn create_temp_project_dir() -> std::path::PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let project_name = format!("test_project_{}_{}", std::process::id(), timestamp);
        
        let temp_dir = env::temp_dir().join(project_name);
        // Clean up if it exists
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");
        temp_dir
    }

    // Helper function to create a valid Rust project structure
    fn setup_rust_project(project_dir: &std::path::Path, manifest_name: &str) -> std::path::PathBuf {
        let src_dir = project_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "// empty lib").unwrap();

        let manifest_path = project_dir.join(manifest_name);
        let toml_content = r#"
            [package]
            name = "test-project"
            version = "0.1.0"
            edition = "2021"

            [dependencies]
            serde = "1.0"
            "#;
        fs::write(&manifest_path, toml_content).unwrap();
        manifest_path
    }

    #[test]
    fn valid_manifest_path_returns_metadata() {
        let temp_dir = create_temp_project_dir();
        let manifest_path = setup_rust_project(&temp_dir, "Cargo.toml");

        let result = get_project_metadata(&manifest_path);
        
        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
        
        assert!(result.is_ok(), "Failed to parse metadata: {:?}", result.err());
        
        let metadata = result.unwrap();
        println!("{:?}", metadata.workspace_members);
        // assert_eq!(metadata.packages.len(), 1);
        // assert_eq!(metadata.packages[0].name.to_string(), "test-project");
        // assert_eq!(metadata.packages[0].version.to_string(), "0.1.0");
    }


}