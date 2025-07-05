//! Code quality analysis and project structure checks for Rust projects.
//!
//! This module provides comprehensive static analysis capabilities for Rust codebases,
//! focusing on code quality, best practices, and project structure validation.
//!
//! # Features
//!
//! ## Code Pattern Analysis
//! - Detects potentially problematic patterns like `.unwrap()` and `.expect()` in library code
//! - Identifies debug macros (`println!`, `dbg!`) that should be removed before release
//! - Finds TODO/FIXME/XXX comments that need attention
//! - Supports parallel processing for improved performance on large codebases
//!
//! ## Project Structure Validation
//! - Validates presence of essential files (README.md, LICENSE)
//! - Checks for proper source file organization (src/lib.rs, src/main.rs, src/bin/)
//! - Integrates with Cargo manifest data for context-aware analysis
//!
//! ## Lint Configuration Checks
//! - Verifies presence of recommended `#![deny(...)]` attributes
//! - Configurable through the project's configuration system
//!
//! # Usage
//!
//! ```rust,no_run
//! use std::path::Path;
//! use code_checks::{collect_rust_files, check_code_patterns, check_project_structure};
//!
//! let project_root = Path::new("./my_project");
//! let rust_files = collect_rust_files(project_root);
//! let findings = check_code_patterns(&rust_files, project_root);
//! 
//! for finding in findings {
//!     println!("{}: {}", finding.code, finding.message);
//! }
//! ```
//!
//! The module is designed to integrate seamlessly with cargo-dokita's diagnostic system
//! and configuration management, providing actionable feedback for Rust developers.

use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};
use crate::config::Config;
use crate::diagnostics::{Finding, Severity};
use regex::Regex;
use once_cell::sync::Lazy;
use crate::manifest::CargoManifest; 
use rayon::prelude::*;

static UNWRAP_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\.unwrap\(\)").unwrap());
static EXPECT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\.expect\s*\("#).unwrap());
static PRINTLN_DBG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(println!|dbg!)\s*\(").unwrap());
static TODO_COMMENT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"//\s*(TODO|FIXME|XXX)").unwrap());
static DENY_LINT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"#!\[deny\(([^)]+)\)\]").unwrap());

fn is_rust_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
        && entry.path().extension().is_some_and(|ext| ext == "rs")
}

pub fn collect_rust_files(project_root: &Path) -> Vec<PathBuf> {
    let mut rust_files = Vec::new();
    let source_roots = [
        project_root.join("src"),
        project_root.join("tests"),
        project_root.join("examples"),
        project_root.join("benches"),
    ];

    for root in source_roots.iter().filter(|p| p.exists() && p.is_dir()) {
        WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok) // Ignore errors during walk, or handle them
            .filter(is_rust_file)
            .for_each(|entry| rust_files.push(entry.path().to_path_buf()));
    }
    rust_files
}

fn is_library_file(file_path: &Path, project_root: &Path) -> bool {
    let src_lib_path = project_root.join("src").join("lib.rs");
    
    file_path.starts_with(project_root.join("src"))
        && file_path != src_lib_path // Allow in lib.rs if it's a binary-only crate's "main" logic
        && !file_path.ends_with("main.rs") // Check if it's literally main.rs
        && file_path.components().all(|c| c.as_os_str() != "bin") // Not in a src/bin/ subdirectory
        && file_path != project_root.join("build.rs") // Not build script
}

pub fn check_code_patterns(
    rust_files: &[PathBuf],
    project_root: &Path
) -> Vec<Finding> {
    let findings_from_all_files: Vec<Finding> = rust_files
        .par_iter()
        .flat_map(|file_path_ref| {
            let file_path = &**file_path_ref;
            let mut per_file_findings: Vec<Finding> = Vec::new();

            let is_lib_context = is_library_file(file_path, project_root);


        // Skip build.rs for some checks like unwrap/expect, as they are common there
        if file_path.ends_with("build.rs") {
            // Could have specific checks for build.rs if desired
            // continue; // Or let specific checks decide
        }


        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                per_file_findings.push(Finding::new(
                    "IO001", // File Read Error
                    format!("Failed to read file {:?}: {}", file_path, e),
                    Severity::Warning,
                    Some(file_path.to_string_lossy().into_owned()),
                ));
                return per_file_findings;
            }
        };

        // Iterate over lines to get line numbers for findings
        for (line_num, line_content) in content.lines().enumerate() {
            let line_number_for_finding = line_num + 1; // 1-indexed

            // Check for .unwrap() in library context
            if is_lib_context && UNWRAP_REGEX.is_match(line_content) && !file_path.ends_with("build.rs") {
                per_file_findings.push(Finding::new(
                    "CODE001",
                    "'.unwrap()' used in library context. Consider using '?' or pattern matching.".to_string(),
                    Severity::Warning,
                    Some(file_path.to_string_lossy().into_owned()),
                ).with_line(line_number_for_finding));
            }

            // Check for .expect() in library context
            if is_lib_context && EXPECT_REGEX.is_match(line_content) && !file_path.ends_with("build.rs") {
                per_file_findings.push(Finding::new(
                    "CODE002",
                    "'.expect()' used in library context. While better than unwrap, prefer '?' or specific error handling.".to_string(),
                    Severity::Note, // expect is slightly better than unwrap
                    Some(file_path.to_string_lossy().into_owned()),
                ).with_line(line_number_for_finding));
            }

            // Check for println!/dbg! in library context
            if is_lib_context && PRINTLN_DBG_REGEX.is_match(line_content) && !file_path.ends_with("build.rs") {
                 // Further refine: allow in main fn of examples, benches.
                 // This check is tricky without knowing the exact role of the file.
                 // For now, broad check on `is_lib_context`.
                per_file_findings.push(Finding::new(
                    "CODE003",
                    "Diagnostic macro (println! or dbg!) found in library context. Remove before release.".to_string(),
                    Severity::Note,
                    Some(file_path.to_string_lossy().into_owned()),
                ).with_line(line_number_for_finding));
            }

            // Check for TODO/FIXME comments (applies to all files)
            if TODO_COMMENT_REGEX.is_match(line_content) {
                let comment_type = TODO_COMMENT_REGEX.captures(line_content).unwrap().get(1).unwrap().as_str();
                per_file_findings.push(Finding::new(
                    "CODE004",
                    format!("Found '{}' comment. Address or create an issue for it.", comment_type),
                    Severity::Note,
                    Some(file_path.to_string_lossy().into_owned()),
                ).with_line(line_number_for_finding));
            }
        }

            per_file_findings
        }).collect();
    
    findings_from_all_files
}


pub fn check_project_structure(
    project_root: &Path,
    manifest_data: Option<&CargoManifest>, // Pass the parsed Cargo.toml
    // _metadata: Option<&Metadata>, // Pass metadata, not used yet but good for future
) -> Vec<Finding> {
    let mut findings = Vec::new();

    let has_lib_rs = project_root.join("src").join("lib.rs").is_file();
    let has_main_rs = project_root.join("src").join("main.rs").is_file();
    let has_bin_dir = project_root.join("src").join("bin").is_dir();

    // Heuristic: If there's a [lib] section or no explicit [[bin]] targets and no main.rs,
    // it's likely intended to be a library.
    let is_likely_library = manifest_data.is_some_and(|m| {
        // Check if [lib] path is specified, or if name is specified (implies lib)
        // This part of `toml` parsing for manifest.lib might need to be added to `CargoManifest` struct
        m.package.as_ref().is_some_and(|p| {
            // A common pattern is that the package name is used for the lib if not specified
            // This is still a heuristic. cargo_metadata is better.
            let default_lib_name = p.name.replace('-', "_");
            project_root.join("src").join(format!("{}.rs", default_lib_name)).exists() || has_lib_rs
        })
    });


    if let Some(_pkg) = manifest_data.and_then(|m| m.package.as_ref()) {

        // If it has a `[package]` section and is not a virtual workspace manifest
        if !is_likely_library && !has_main_rs && !has_bin_dir {
            findings.push(Finding::new(
                "STRUCT001",
                "Project has neither src/lib.rs, src/main.rs, nor src/bin/ directory. Is it a virtual workspace or missing source files?".to_string(),
                Severity::Warning,
                Some("Cargo.toml".to_string()), // Related to project structure defined by Cargo.toml implicitly
            ));
        }

        // If it has main.rs, it's likely a binary.
        // If it has lib.rs, it's likely a library.
        // If it has both, it's a common pattern for a crate that is both a lib and has a default binary.

        // Check for README.md (could also be in manifest checks if `readme` field points to it)
        let readme_path_md = project_root.join("README.md");
        let readme_path_rst = project_root.join("README.rst"); // Less common in Rust but possible
        if !readme_path_md.is_file() && !readme_path_rst.is_file() {
             // Check if Cargo.toml specifies `readme = false` or a custom readme file
            let readme_specified_and_false_or_exists = manifest_data
                .and_then(|m| m.package.as_ref())
                .and_then(|p| p.readme.as_ref())
                .is_some_and(|r_val| {
                    // if r_val is a bool and false, then it's fine
                    if let Some(b) = r_val.as_bool() { // Assuming readme can be bool or string
                        !b
                    } else if let Some(s) = r_val.as_str() {
                        project_root.join(s).is_file()
                    } else {
                        false // Not a bool or string, odd.
                    }
                });

            if !readme_specified_and_false_or_exists {
                findings.push(Finding::new(
                    "STRUCT002",
                    "Missing README.md file in project root. Consider adding one.".to_string(),
                    Severity::Note,
                    Some(readme_path_md.to_string_lossy().into_owned()),
                ));
            }
        }

        // Check for LICENSE file (could also be in manifest checks if `license-file` points to it)
        // Common names: LICENSE, LICENSE.txt, LICENSE-MIT, LICENSE-APACHE, COPYING
        let license_files = ["LICENSE", "LICENSE.txt", "LICENSE-MIT", "LICENSE-APACHE", "COPYING", "UNLICENSE"];
        let has_license_file = license_files.iter().any(|name| project_root.join(name).is_file() || project_root.join(name.to_uppercase()).is_file() || project_root.join(name.to_lowercase()).is_file());

        if !has_license_file {
            // Check if Cargo.toml specifies `license-file`
            let license_file_specified = manifest_data
                .and_then(|m| m.package.as_ref())
                .is_some_and(|p| {
                    // Assuming you add `license_file: Option<String>` to your Package struct
                    // p.license_file.as_ref().map_or(false, |lf| project_root.join(lf).is_file())
                    // For now, let's assume it's not specified if Package struct doesn't have it
                    p.license.is_some() && p.license.as_deref() != Some("") // If license is specified, a file is good practice
                });

            if !license_file_specified || manifest_data.and_then(|m| m.package.as_ref()).is_none_or(|p| p.license.is_none()) {
                 findings.push(Finding::new(
                    "STRUCT003",
                    "Missing LICENSE file in project root. Consider adding one (e.g., LICENSE-MIT or LICENSE-APACHE).".to_string(),
                    Severity::Warning, // More important than README
                    Some("Project Root".to_string()), // Generic path
                ));
            }
        }
    }

    findings
}

// src/code_checks.rs

pub fn check_missing_denied_lints(
    project_root: &Path,
    config: &Config, // Config could specify which lints are recommended
) -> Vec<Finding> {
    let mut findings = Vec::new();
    // Example: Check src/lib.rs if it exists
    let lib_rs_path = project_root.join("src/lib.rs");
    let main_rs_path = project_root.join("src/main.rs");
    let files_to_check = [&lib_rs_path, &main_rs_path];

    let recommended_denials = vec!["warnings"]; // Could come from config
                                            // or clippy::all, clippy::pedantic

    for file_path in files_to_check.iter().filter(|p| p.exists()) {
        if !config.is_check_enabled("LINT001") { continue; } // Example check code

        if let Ok(content) = fs::read_to_string(file_path) {
            let mut found_denials = std::collections::HashSet::new();
            for cap in DENY_LINT_REGEX.captures_iter(&content) {
                // cap[1] contains the comma-separated list of lints
                for lint in cap[1].split(',').map(|s| s.trim()) {
                    found_denials.insert(lint.to_string());
                }
            }

            for rec_denial in &recommended_denials {
                if !found_denials.contains(*rec_denial) {
                    findings.push(Finding::new(
                        "LINT001",
                        format!("Consider adding `#![deny({})]` to the top of {:?} for stricter linting.", rec_denial, file_path.file_name().unwrap_or_default()),
                        Severity::Note,
                        Some(file_path.to_string_lossy().into_owned())
                    ));
                }
            }
        }
    }
    findings
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use crate::config::{Config, ChecksConfig};
    use crate::diagnostics::{Severity};
    use crate::manifest::{CargoManifest, Package};
    use std::collections::HashMap;

    // Helper function to create a temporary directory with test files
    fn create_test_dir() -> TempDir {
        TempDir::new().expect("Failed to create temp dir")
    }

    // Helper function to create a basic config
    fn create_test_config() -> Config {
        let mut enabled_checks = HashMap::new();
        enabled_checks.insert("LINT001".to_string(), true);
        
        Config {
            general: Default::default(),
            checks: ChecksConfig {
                enabled: enabled_checks,
            },
        }
    }

    // Helper function to create a basic CargoManifest
    fn create_test_manifest(name: &str) -> CargoManifest {
        CargoManifest {
            package: Some(Package {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                license: Some("MIT".to_string()),
                readme: None,
                description: Some("Test package".to_string()),
                edition: Some("2021".to_string()),
                repository: None,
            }),
            dependencies: None,
            dev_dependencies: None,
            build_dependencies: None,
        }
    }

    #[test]
    fn test_is_rust_file() {
        let temp_dir = create_test_dir();
        let rust_file = temp_dir.path().join("test.rs");
        let non_rust_file = temp_dir.path().join("test.txt");
        
        fs::write(&rust_file, "fn main() {}").unwrap();
        fs::write(&non_rust_file, "hello").unwrap();
        
        let rust_entry = WalkDir::new(&rust_file).into_iter().next().unwrap().unwrap();
        let non_rust_entry = WalkDir::new(&non_rust_file).into_iter().next().unwrap().unwrap();
        
        assert!(is_rust_file(&rust_entry));
        assert!(!is_rust_file(&non_rust_entry));
    }

    #[test]
    fn test_collect_rust_files() {
        let temp_dir = create_test_dir();
        let project_root = temp_dir.path();
        
        // Create directory structure
        let src_dir = project_root.join("src");
        let tests_dir = project_root.join("tests");
        let examples_dir = project_root.join("examples");
        
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&tests_dir).unwrap();
        fs::create_dir_all(&examples_dir).unwrap();
        
        // Create Rust files
        fs::write(src_dir.join("lib.rs"), "// lib content").unwrap();
        fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
        fs::write(tests_dir.join("integration_test.rs"), "// test content").unwrap();
        fs::write(examples_dir.join("example.rs"), "fn main() {}").unwrap();
        
        // Create non-Rust file (should be ignored)
        fs::write(src_dir.join("config.toml"), "[package]").unwrap();
        
        let rust_files = collect_rust_files(project_root);
        
        assert_eq!(rust_files.len(), 4);
        assert!(rust_files.iter().any(|p| p.ends_with("lib.rs")));
        assert!(rust_files.iter().any(|p| p.ends_with("main.rs")));
        assert!(rust_files.iter().any(|p| p.ends_with("integration_test.rs")));
        assert!(rust_files.iter().any(|p| p.ends_with("example.rs")));
    }

    #[test]
    fn test_is_library_file() {
        let temp_dir = create_test_dir();
        let project_root = temp_dir.path();
        let src_dir = project_root.join("src");
        let bin_dir = src_dir.join("bin");
        
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        
        let lib_rs = src_dir.join("lib.rs");
        let main_rs = src_dir.join("main.rs");
        let module_rs = src_dir.join("module.rs");
        let bin_file = bin_dir.join("binary.rs");
        let build_rs = project_root.join("build.rs");
        
        // Test various file types
        assert!(!is_library_file(&lib_rs, project_root)); // lib.rs is allowed
        assert!(!is_library_file(&main_rs, project_root)); // main.rs is not library
        assert!(is_library_file(&module_rs, project_root)); // module in src/ is library
        assert!(!is_library_file(&bin_file, project_root)); // bin/ files are not library
        assert!(!is_library_file(&build_rs, project_root)); // build.rs is not library
    }

    #[test]
    fn test_check_code_patterns_unwrap_detection() {
        let temp_dir = create_test_dir();
        let project_root = temp_dir.path();
        let src_dir = project_root.join("src");
        
        fs::create_dir_all(&src_dir).unwrap();
        
        let test_file = src_dir.join("module.rs");
        let test_content = r#"
fn test_function() {
    let result = some_operation().unwrap(); // This should be flagged
    let other = another_op().expect("failed"); // This should be flagged too
    println!("Debug output"); // This should be flagged in library context
    // TODO: Fix this later // This should be flagged
}
"#;
        fs::write(&test_file, test_content).unwrap();
        
        let rust_files = vec![test_file];
        let findings = check_code_patterns(&rust_files, project_root);
        
        // Should find unwrap, expect, println, and TODO
        assert!(findings.iter().any(|f| f.code == "CODE001")); // unwrap
        assert!(findings.iter().any(|f| f.code == "CODE002")); // expect
        assert!(findings.iter().any(|f| f.code == "CODE003")); // println
        assert!(findings.iter().any(|f| f.code == "CODE004")); // TODO
        
        // Check that line numbers are correct
        let unwrap_finding = findings.iter().find(|f| f.code == "CODE001").unwrap();
        assert_eq!(unwrap_finding.line_number, Some(3));
    }

    #[test]
    fn test_check_code_patterns_build_script_exclusion() {
        let temp_dir = create_test_dir();
        let project_root = temp_dir.path();
        
        let build_rs = project_root.join("build.rs");
        let build_content = r#"
fn main() {
    let result = some_operation().unwrap(); // This should NOT be flagged in build.rs
    println!("Building..."); // This should NOT be flagged in build.rs
}
"#;
        fs::write(&build_rs, build_content).unwrap();
        
        let rust_files = vec![build_rs];
        let findings = check_code_patterns(&rust_files, project_root);
        
        // Should not find CODE001, CODE002, or CODE003 in build.rs
        assert!(!findings.iter().any(|f| f.code == "CODE001"));
        assert!(!findings.iter().any(|f| f.code == "CODE002"));
        assert!(!findings.iter().any(|f| f.code == "CODE003"));
        
        // But should still find TODO comments
        // (if we add one to the test content)
    }

    #[test]
    fn test_check_code_patterns_file_read_error() {
        let temp_dir = create_test_dir();
        let project_root = temp_dir.path();
        
        // Create a path to a non-existent file
        let non_existent_file = project_root.join("nonexistent.rs");
        let rust_files = vec![non_existent_file];
        
        let findings = check_code_patterns(&rust_files, project_root);
        
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "IO001");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn test_check_project_structure_missing_source_files() {
        let temp_dir = create_test_dir();
        let project_root = temp_dir.path();
        
        // Create Cargo.toml but no source files
        let manifest = create_test_manifest("test-project");
        
        let findings = check_project_structure(project_root, Some(&manifest));
        
        assert!(findings.iter().any(|f| f.code == "STRUCT001"));
    }

    #[test]
    fn test_check_project_structure_missing_readme() {
        let temp_dir = create_test_dir();
        let project_root = temp_dir.path();
        let src_dir = project_root.join("src");
        
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
        
        let manifest = create_test_manifest("test-project");
        
        let findings = check_project_structure(project_root, Some(&manifest));
        
        assert!(findings.iter().any(|f| f.code == "STRUCT002"));
    }

    #[test]
    fn test_check_project_structure_missing_license() {
        let temp_dir = create_test_dir();
        let project_root = temp_dir.path();
        let src_dir = project_root.join("src");
        
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
        
        let mut manifest = create_test_manifest("test-project");
        manifest.package.as_mut().unwrap().license = None; // No license specified
        
        let findings = check_project_structure(project_root, Some(&manifest));
        
        assert!(findings.iter().any(|f| f.code == "STRUCT003"));
    }

    #[test]
    fn test_check_project_structure_with_readme_and_license() {
        let temp_dir = create_test_dir();
        let project_root = temp_dir.path();
        let src_dir = project_root.join("src");
        
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
        fs::write(project_root.join("README.md"), "# Test Project").unwrap();
        fs::write(project_root.join("LICENSE"), "MIT License").unwrap();
        
        let manifest = create_test_manifest("test-project");
        
        let findings = check_project_structure(project_root, Some(&manifest));
        
        // Should not flag missing README or LICENSE
        assert!(!findings.iter().any(|f| f.code == "STRUCT002"));
        assert!(!findings.iter().any(|f| f.code == "STRUCT003"));
    }

    #[test]
    fn test_check_missing_denied_lints_missing_warnings() {
        let temp_dir = create_test_dir();
        let project_root = temp_dir.path();
        let src_dir = project_root.join("src");
        
        fs::create_dir_all(&src_dir).unwrap();
        
        let lib_rs = src_dir.join("lib.rs");
        let lib_content = r#"
// No deny attributes here
pub fn hello() {
    println!("Hello, world!");
}
"#;
        fs::write(&lib_rs, lib_content).unwrap();
        
        let config = create_test_config();
        let findings = check_missing_denied_lints(project_root, &config);
        
        assert!(findings.iter().any(|f| f.code == "LINT001"));
    }

    #[test]
    fn test_check_missing_denied_lints_has_warnings() {
        let temp_dir = create_test_dir();
        let project_root = temp_dir.path();
        let src_dir = project_root.join("src");
        
        fs::create_dir_all(&src_dir).unwrap();
        
        let lib_rs = src_dir.join("lib.rs");
        let lib_content = r#"
#![deny(warnings)]

pub fn hello() {
    println!("Hello, world!");
}
"#;
        fs::write(&lib_rs, lib_content).unwrap();
        
        let config = create_test_config();
        let findings = check_missing_denied_lints(project_root, &config);
        
        // Should not flag missing warnings denial
        assert!(!findings.iter().any(|f| f.code == "LINT001"));
    }

    #[test]
    fn test_check_missing_denied_lints_disabled_check() {
        let temp_dir = create_test_dir();
        let project_root = temp_dir.path();
        let src_dir = project_root.join("src");
        
        fs::create_dir_all(&src_dir).unwrap();
        
        let lib_rs = src_dir.join("lib.rs");
        fs::write(&lib_rs, "pub fn hello() {}").unwrap();
        
        // Create config with LINT001 disabled
        let mut config = Config::default();
        config.checks.enabled.insert("LINT001".to_string(), false);
        
        let findings = check_missing_denied_lints(project_root, &config);
        
        // Should not flag anything when check is disabled
        assert!(!findings.iter().any(|f| f.code == "LINT001"));
    }

    #[test]
    fn test_regex_patterns() {
        // Test the static regex patterns
        assert!(UNWRAP_REGEX.is_match("result.unwrap()"));
        assert!(UNWRAP_REGEX.is_match("some_func().unwrap()"));
        assert!(!UNWRAP_REGEX.is_match("unwrap_or_default()"));
        
        assert!(EXPECT_REGEX.is_match(r#"result.expect("failed")"#));
        assert!(EXPECT_REGEX.is_match("result.expect("));
        assert!(!EXPECT_REGEX.is_match("expected_value"));
        
        assert!(PRINTLN_DBG_REGEX.is_match("println!(\"hello\")"));
        assert!(PRINTLN_DBG_REGEX.is_match("dbg!(value)"));
        assert!(!PRINTLN_DBG_REGEX.is_match("print_value()"));
        
        assert!(TODO_COMMENT_REGEX.is_match("// TODO: fix this"));
        assert!(TODO_COMMENT_REGEX.is_match("// FIXME: broken"));
        assert!(TODO_COMMENT_REGEX.is_match("//XXX urgent"));
        assert!(!TODO_COMMENT_REGEX.is_match("// NOTE: this is fine"));
        
        assert!(DENY_LINT_REGEX.is_match("#![deny(warnings)]"));
        assert!(DENY_LINT_REGEX.is_match("#![deny(clippy::all, warnings)]"));
    }

    #[test]
    fn test_parallel_processing() {
        let temp_dir = create_test_dir();
        let project_root = temp_dir.path();
        let src_dir = project_root.join("src");
        
        fs::create_dir_all(&src_dir).unwrap();
        
        // Create multiple files to test parallel processing
        for i in 0..10 {
            let file_path = src_dir.join(format!("module_{}.rs", i));
            let content = format!(r#"
// TODO: implement module {}
pub fn function_{}() {{
    let result = some_operation().unwrap();
}}
"#, i, i);
            fs::write(&file_path, content).unwrap();
        }
        
        let rust_files = collect_rust_files(project_root);
        let findings = check_code_patterns(&rust_files, project_root);
        
        // Should find issues in all files
        assert_eq!(findings.iter().filter(|f| f.code == "CODE001").count(), 10); // unwrap
        assert_eq!(findings.iter().filter(|f| f.code == "CODE004").count(), 10); // TODO
    }
}