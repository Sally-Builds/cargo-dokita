use std::{fs, path::PathBuf, process};
use dependency_analysis::check_vulnerability;
use reqwest::blocking::Client as HttpClient;
use std::io::Write; // For termcolor
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use rayon::join;
use diagnostics::{Finding,Severity};
pub mod manifest;
pub mod diagnostics;
pub mod dependency_analysis;
pub mod crates_io_api;
pub mod code_checks;
pub mod config;

#[derive(Debug)]
pub enum MyError {
    NotRustProject,
    UnresolvableProjectPath,
}


pub fn analyze_project (project_path: &str, output_format: &str) -> Result<(), MyError> {
    let mut findings: Vec<Finding> = Vec::new();
    let project_path = match fs::canonicalize(project_path) {
        Ok(path) => path,
        Err(e) => {
            eprint!("Error Could not resolve project path - {:?}", e);
            return Err(MyError::UnresolvableProjectPath)
        },
    };

    let mut stdout = StandardStream::stdout(ColorChoice::Auto);


    let config = match config::Config::load_from_project_root(&project_path) {
        Ok(cfg) => {
            if  project_path.join(config::CONFIG_FILE_NAME).exists() {
                println!("Loaded configuration from {}", config::CONFIG_FILE_NAME);
            }
            cfg
        }
        Err(e) => {
            println!("Warning: Could not load or parse {}: {}. Using default configuration.", config::CONFIG_FILE_NAME, e);
            // Optionally add a Finding for bad config
            config::Config::default()
        }
    };

    let rust_files = code_checks::collect_rust_files(&project_path.as_path());
    findings.extend(code_checks::check_code_patterns(&rust_files, &project_path));

    if !is_rust_project(&project_path) {
        eprintln!("This is not a rust project");
        return Err(MyError::NotRustProject);
    }

    let cargo_toml_path = project_path.join("Cargo.toml");

    let cargo_manifest = manifest::CargoManifest::parse(&cargo_toml_path.as_path());

    match &cargo_manifest {
        Ok(data) => {
            findings.extend(code_checks::check_project_structure(&project_path, Some(data)));
        },
        Err(_) => {}
    }


    let http_client = HttpClient::new();

    let (manifest_findings, dep_findings) = rayon::join(|| {
        let mut f = Vec::new();
            match cargo_manifest { // md is Option<CargoManifest>
                Ok(md) => {
                        f.extend(manifest::check_missing_metadata(&md, &config));
                        f.extend(manifest::check_dependency_versions(&md, &config));
                        f.extend(manifest::check_rust_edition(&md, &config));
                    },
                Err(_) => {

                }

            } 
        f
    }, || {
        let mut f = Vec::new();
        match dependency_analysis::get_project_metadata(&cargo_toml_path.as_path()) {
                Ok(metadata) => {
                    let outdated_dependencies_findings = dependency_analysis::check_outdated_dependencies(&metadata, &http_client);
                    f.extend(outdated_dependencies_findings);
                },
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        let vulnerability_findings = check_vulnerability(project_path.as_path());
        f.extend(vulnerability_findings);
        f
    });

    findings.extend(manifest_findings);
    findings.extend(dep_findings);

    if findings.is_empty() {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).unwrap_or_default();
        writeln!(&mut stdout, "No issues found. Your project looks healthy (based on current checks)!").unwrap_or_default();
        stdout.reset().unwrap_or_default();
    } else {
        if output_format == "json" {
            match serde_json::to_string_pretty(&findings) {
                Ok(json_output) => println!("{}", json_output),
                Err(e) => {
                    eprintln!("Error serializing findings to JSON: {}", e);
                    process::exit(1);
                }
            }
        }else {
            for finding in &findings {
                // Basic output, can be improved with termcolor later
                let severity_str = match finding.severity {
                    Severity::Error => "ERROR",
                    Severity::Warning => "WARNING",
                    Severity::Note => "NOTE",
                };

                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true)).unwrap_or_default();
                write!(&mut stdout, "[{}]", severity_str).unwrap_or_default();
                stdout.reset().unwrap_or_default();

                let file_info = finding.file_path.as_deref().unwrap_or("N/A");
                let line_info = finding.line_number.map_or("".to_string(), |l| {
                    format!("{}", l)
                });

                stdout.set_color(ColorSpec::new().set_fg(Some(Color::Magenta))).unwrap_or_default();
                write!(&mut stdout, " ({})", finding.code).unwrap_or_default();
                stdout.reset().unwrap_or_default();

                writeln!(&mut stdout, ": {} [{}{}]", finding.message, file_info, line_info).unwrap_or_default();
            }
        }

        writeln!(&mut stdout, "\nFound {} issues:", findings.len()).unwrap_or_default();
    }

    if findings.iter().any(|f| matches!(f.severity, Severity::Error | Severity::Warning)) {
        process::exit(1);
    }

    Ok(())
}

fn is_rust_project(project_path: &PathBuf) -> bool {
    if !project_path.is_dir() {
        return false
    }

    project_path.join("Cargo.toml").is_file()
    

}




mod tests {}