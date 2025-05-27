use std::{fs, path::PathBuf, process};

use diagnostics::{Finding,Severity};
pub mod manifest;
pub mod diagnostics;

#[derive(Debug)]
pub enum MyError {
    NotRustProject,
    UnresolvableProjectPath,
}


pub fn analyze_project (project_path: &str) -> Result<(), MyError> {
    let mut findings: Vec<Finding> = Vec::new();
    let project_path = match fs::canonicalize(project_path) {
        Ok(path) => path,
        Err(e) => {
            eprint!("Error Could not resolve project path - {:?}", e);
            return Err(MyError::UnresolvableProjectPath)
        },
    };

    if !is_rust_project(&project_path) {
        eprintln!("This is not a rust project");
        return Err(MyError::NotRustProject);
    }

    let project_path = project_path.join("Cargo.toml");

    let cargo_manifest = manifest::CargoManifest::v(&project_path.as_path());

    match &cargo_manifest {
        Ok(data) => {
            findings.extend(manifest::check_missing_metadata(&data));
            findings.extend(manifest::check_dependency_versions(&data));
            findings.extend(manifest::check_rust_edition(&data));
        },
        Err(e) => {

        }
    }

    // println!("All good!!!, {:#?}", findings);


    if findings.is_empty() {
        println!("No issues found. Your project looks healthy (based on current checks)!");
    } else {
        println!("\nFound {} issues:", findings.len());
        for finding in findings {
            // Basic output, can be improved with termcolor later
            let severity_str = match finding.severity {
                Severity::Error => "ERROR",
                Severity::Warning => "WARNING",
                Severity::Note => "NOTE",
            };
            let file_info = finding.file_path.as_deref().unwrap_or("N/A");
            println!(
                "[{}] ({}): {} [{}]",
                severity_str, finding.code, finding.message, file_info
            );
        }
        // Exit with an error code if there are errors or warnings
        // if findings.iter().any(|f| matches!(f.severity, Severity::Error | Severity::Warning)) {
        //     process::exit(1);
        // }
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