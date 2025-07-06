//! # Cargo Dokita
//!
//! A comprehensive Rust project analysis tool that performs static analysis on Rust projects
//! to identify potential issues, security vulnerabilities, and code quality problems.
//!
//! ## Features
//!
//! - **Dependency Analysis**: Check for outdated dependencies and known security vulnerabilities
//! - **Code Quality Checks**: Analyze code patterns and project structure
//! - **Manifest Validation**: Validate Cargo.toml metadata and configuration
//! - **Configurable Rules**: Support for custom configuration through `.dokita.toml` files
//! - **Multiple Output Formats**: Support for both human-readable and JSON output
//!
//! ## Usage
//!
//! The main entry point for analysis is the [`analyze_project`] function:
//!
//! ```rust,no_run
//! use cargo_dokita::analyze_project;
//!
//! // Analyze a Rust project with default text output
//! match analyze_project("./my-rust-project", "text") {
//!     Ok(()) => println!("Analysis completed successfully"),
//!     Err(e) => eprintln!("Analysis failed: {:?}", e),
//! }
//! ```
//!
//! ## Modules
//!
//! - [`manifest`] - Cargo.toml parsing and validation
//! - [`diagnostics`] - Core diagnostic types and severity levels
//! - [`dependency_analysis`] - Dependency checking and vulnerability scanning
//! - [`crates_io_api`] - Integration with crates.io API
//! - [`code_checks`] - Static code analysis and pattern detection
//! - [`config`] - Configuration file handling and settings

// filepath: /home/sally-nwamama/Desktop/rust_projects/cargo-dokita/src/lib.rs
use dependency_analysis::check_vulnerability;
use diagnostics::{Finding, Severity};
use reqwest::blocking::Client as HttpClient;
use std::io::Write; // For termcolor
use std::{fs, path::Path, process};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Cargo.toml manifest parsing and validation functionality.
pub mod manifest;

/// Core diagnostic types, severity levels, and finding structures.
pub mod diagnostics;

/// Dependency analysis including vulnerability scanning and outdated package detection.
pub mod dependency_analysis;

/// Integration with the crates.io API for package information retrieval.
pub mod crates_io_api;

/// Static code analysis and project structure validation.
pub mod code_checks;

/// Configuration file handling and project settings management.
pub mod config;

/// Error types that can occur during project analysis.
#[derive(Debug)]
pub enum MyError {
    /// The specified directory is not a valid Rust project (missing Cargo.toml).
    NotRustProject,
    /// The provided project path could not be resolved or canonicalized.
    UnresolvableProjectPath,
    /// Analysis completed but found issues. Contains the list of findings for test purposes.
    HasIssues(Vec<Finding>), // For test purposes
}

/// Analyzes a Rust project for potential issues and vulnerabilities.
///
/// This function performs a comprehensive analysis of a Rust project, including:
/// - Dependency vulnerability scanning
/// - Code pattern analysis
/// - Project structure validation
/// - Manifest (Cargo.toml) checks
/// - Configuration validation
///
/// # Arguments
///
/// * `project_path` - Path to the root directory of the Rust project to analyze
/// * `output_format` - Output format for results ("json" for JSON output, anything else for human-readable text)
///
/// # Returns
///
/// Returns `Ok(())` if analysis completes successfully (even if issues are found).
/// Returns `Err(MyError)` if:
/// - The project path cannot be resolved ([`MyError::UnresolvableProjectPath`])
/// - The directory is not a valid Rust project ([`MyError::NotRustProject`])
///
/// # Behavior
///
/// - If no issues are found, prints a success message in green
/// - If issues are found, outputs them according to the specified format
/// - Calls `process::exit(1)` if any errors or warnings are found
/// - Supports parallel execution of some analysis phases for improved performance
///
/// # Examples
///
/// ```rust,no_run
/// use cargo_dokita::analyze_project;
///
/// // Analyze with text output
/// analyze_project("./my-project", "human").unwrap();
///
/// // Analyze with JSON output
/// analyze_project("./my-project", "json").unwrap();
/// ```
///
/// # Panics
///
/// This function may panic if there are issues with terminal color output,
/// but such panics are handled gracefully with `unwrap_or_default()`.
pub fn analyze_project(project_path: &str, output_format: &str) -> Result<(), MyError> {
    let mut findings: Vec<Finding> = Vec::new();
    let project_path = match fs::canonicalize(project_path) {
        Ok(path) => path,
        Err(e) => {
            eprint!("Error Could not resolve project path - {e:?}");
            return Err(MyError::UnresolvableProjectPath);
        }
    };

    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    let config = match config::Config::load_from_project_root(&project_path) {
        Ok(cfg) => {
            if project_path.join(config::CONFIG_FILE_NAME).exists() {
                println!("Loaded configuration from {}", config::CONFIG_FILE_NAME);
            }
            cfg
        }
        Err(e) => {
            println!(
                "Warning: Could not load or parse {}: {}. Using default configuration.",
                config::CONFIG_FILE_NAME,
                e
            );
            // Optionally add a Finding for bad config
            config::Config::default()
        }
    };

    let rust_files = code_checks::collect_rust_files(project_path.as_path());
    findings.extend(code_checks::check_code_patterns(&rust_files, &project_path));

    if !is_rust_project(&project_path) {
        eprintln!("This is not a rust project");
        return Err(MyError::NotRustProject);
    }

    let cargo_toml_path = project_path.join("Cargo.toml");

    let cargo_manifest = manifest::CargoManifest::parse(cargo_toml_path.as_path());

    if let Ok(data) = &cargo_manifest {
        findings.extend(code_checks::check_project_structure(
            &project_path,
            Some(data),
        ));
    }

    let http_client = HttpClient::new();

    let (manifest_findings, dep_findings) = rayon::join(
        || {
            let mut f = Vec::new();
            // md is Option<CargoManifest>
            if let Ok(md) = cargo_manifest {
                f.extend(manifest::check_missing_metadata(&md, &config));
                f.extend(manifest::check_dependency_versions(&md, &config));
                f.extend(manifest::check_rust_edition(&md));
            }
            f
        },
        || {
            let mut f = Vec::new();
            match dependency_analysis::get_project_metadata(cargo_toml_path.as_path()) {
                Ok(metadata) => {
                    let outdated_dependencies_findings =
                        dependency_analysis::check_outdated_dependencies(&metadata, &http_client);
                    f.extend(outdated_dependencies_findings);
                }
                Err(e) => {
                    println!("{e:?}");
                }
            }
            let vulnerability_findings = check_vulnerability(project_path.as_path());
            f.extend(vulnerability_findings);
            f
        },
    );

    findings.extend(manifest_findings);
    findings.extend(dep_findings);

    findings.extend(code_checks::check_missing_denied_lints(
        project_path.as_path(),
        &config,
    ));

    if findings.is_empty() {
        stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Green)))
            .unwrap_or_default();
        writeln!(
            &mut stdout,
            "No issues found. Your project looks healthy (based on current checks)!"
        )
        .unwrap_or_default();
        stdout.reset().unwrap_or_default();
    } else {
        if output_format == "json" {
            match serde_json::to_string_pretty(&findings) {
                Ok(json_output) => println!("{json_output}",),
                Err(e) => {
                    eprintln!("Error serializing findings to JSON: {e:?}");
                    process::exit(1);
                }
            }
        } else {
            for finding in &findings {
                // Basic output, can be improved with termcolor later
                let severity_str = match finding.severity {
                    Severity::Error => "ERROR",
                    Severity::Warning => "WARNING",
                    Severity::Note => "NOTE",
                };

                stdout
                    .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))
                    .unwrap_or_default();
                write!(&mut stdout, "[{severity_str}]").unwrap_or_default();
                stdout.reset().unwrap_or_default();

                let file_info = finding.file_path.as_deref().unwrap_or("N/A");
                let line_info = finding
                    .line_number
                    .map_or("".to_string(), |l| format!("{l}"));

                stdout
                    .set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))
                    .unwrap_or_default();
                write!(&mut stdout, " ({})", finding.code).unwrap_or_default();
                stdout.reset().unwrap_or_default();

                writeln!(
                    &mut stdout,
                    ": {} [{}{}]",
                    finding.message, file_info, line_info
                )
                .unwrap_or_default();
            }
        }

        writeln!(&mut stdout, "\nFound {} issues:", findings.len()).unwrap_or_default();
    }

    if findings
        .iter()
        .any(|f| matches!(f.severity, Severity::Error | Severity::Warning))
    {
        process::exit(1);
    }

    Ok(())
}

/// Test-friendly version of [`analyze_project`] that returns findings instead of calling `process::exit`.
///
/// This function performs the same analysis as [`analyze_project`] but is designed for use in tests
/// and other scenarios where you need programmatic access to the findings without side effects.
///
/// # Arguments
///
/// * `project_path` - Path to the root directory of the Rust project to analyze
/// * `_output_format` - Output format parameter (currently unused in this function)
///
/// # Returns
///
/// Returns:
/// - `Ok(Vec<Finding>)` - Analysis completed successfully with the list of findings (may be empty)
/// - `Err(MyError::UnresolvableProjectPath)` - The project path could not be resolved
/// - `Err(MyError::NotRustProject)` - The directory is not a valid Rust project
///
/// # Differences from `analyze_project`
///
/// - Does not print output to stdout/stderr
/// - Does not call `process::exit()`
/// - Returns findings as a vector for programmatic inspection
/// - Suitable for use in unit tests and integration tests
///
/// # Examples
///
/// ```rust,no_run
/// use cargo_dokita::analyze_project_for_test;
///
/// match analyze_project_for_test("./test-project", "json") {
///     Ok(findings) => {
///         println!("Found {} issues", findings.len());
///         for finding in findings {
///             println!("Issue: {}", finding.message);
///         }
///     },
///     Err(e) => eprintln!("Analysis failed: {:?}", e),
/// }
/// ```
pub fn analyze_project_for_test(
    project_path: &str,
    _output_format: &str,
) -> Result<Vec<Finding>, MyError> {
    let mut findings: Vec<Finding> = Vec::new();
    let project_path = match fs::canonicalize(project_path) {
        Ok(path) => path,
        Err(e) => {
            eprint!("Error Could not resolve project path - {e:?}");
            return Err(MyError::UnresolvableProjectPath);
        }
    };

    let config = config::Config::load_from_project_root(&project_path).unwrap_or_default();

    // Code checks first (before checking if it's a Rust project)
    let rust_files = code_checks::collect_rust_files(project_path.as_path());
    findings.extend(code_checks::check_code_patterns(&rust_files, &project_path));

    if !is_rust_project(&project_path) {
        return Err(MyError::NotRustProject);
    }

    let cargo_toml_path = project_path.join("Cargo.toml");
    let cargo_manifest = manifest::CargoManifest::parse(cargo_toml_path.as_path());

    if let Ok(data) = &cargo_manifest {
        findings.extend(code_checks::check_project_structure(
            &project_path,
            Some(data),
        ));
    }

    let http_client = HttpClient::new();

    let (manifest_findings, dep_findings) = rayon::join(
        || {
            let mut f = Vec::new();
            if let Ok(md) = cargo_manifest {
                f.extend(manifest::check_missing_metadata(&md, &config));
                f.extend(manifest::check_dependency_versions(&md, &config));
                f.extend(manifest::check_rust_edition(&md));
            }
            f
        },
        || {
            let mut f = Vec::new();
            if let Ok(metadata) =
                dependency_analysis::get_project_metadata(cargo_toml_path.as_path())
            {
                let outdated_dependencies_findings =
                    dependency_analysis::check_outdated_dependencies(&metadata, &http_client);
                f.extend(outdated_dependencies_findings);
            }
            let vulnerability_findings = check_vulnerability(project_path.as_path());
            f.extend(vulnerability_findings);
            f
        },
    );

    findings.extend(manifest_findings);
    findings.extend(dep_findings);
    findings.extend(code_checks::check_missing_denied_lints(
        project_path.as_path(),
        &config,
    ));

    Ok(findings)
}

/// Checks if the given path represents a valid Rust project.
///
/// A directory is considered a valid Rust project if:
/// 1. The path points to an existing directory
/// 2. The directory contains a `Cargo.toml` file
///
/// # Arguments
///
/// * `project_path` - Path to the directory to check
///
/// # Returns
///
/// Returns `true` if the path is a valid Rust project directory, `false` otherwise.
///
/// # Examples
///
/// ```rust,no_run
/// use std::path::PathBuf;
/// # use cargo_dokita::*;
///
/// let valid_project = PathBuf::from("./my-rust-project");
/// let invalid_project = PathBuf::from("./not-a-rust-project");
///
/// // This would be true if ./my-rust-project contains Cargo.toml
/// // let is_valid = is_rust_project(&valid_project);
/// ```
fn is_rust_project(project_path: &Path) -> bool {
    if !project_path.is_dir() {
        return false;
    }

    project_path.join("Cargo.toml").is_file()
}

/// Unit tests for the library functionality.
///
/// This module contains tests for the core analysis functions and helper utilities.
/// Tests use the [`analyze_project_for_test`] function to avoid side effects.
#[cfg(test)]
mod tests {}
