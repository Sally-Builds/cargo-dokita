use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};
use crate::diagnostics::{Finding, Severity};
use regex::Regex;
use once_cell::sync::Lazy;
use crate::manifest::CargoManifest; 
use cargo_metadata::Metadata;

static UNWRAP_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\.unwrap\(\)").unwrap());
static EXPECT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\.expect\s*\("#).unwrap());
static PRINTLN_DBG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(println!|dbg!)\s*\(").unwrap());
static TODO_COMMENT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"//\s*(TODO|FIXME|XXX)").unwrap());

fn is_rust_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file()
        && entry.path().extension().map_or(false, |ext| ext == "rs")
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
// A more robust `is_library_file` would take `&manifest::CargoManifest` or `&cargo_metadata::Metadata`
// to determine if the project *has* a library target. For now, this heuristic is okay.


pub fn check_code_patterns(
    rust_files: &[PathBuf],
    project_root: &Path
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for file_path in rust_files {
        // Determine if the file is part of application code (main, examples, tests) vs library code
        // Heuristic: if it's in src/ but not main.rs and not in src/bin/, it's "library-like"
        // A more robust check would use cargo_metadata to see if there's a lib target.

        // For now, let's use a simplified `is_library_file` heuristic:
        // let is_lib_context = file_path.starts_with(project_root.join("src"))
        //     && !file_path.ends_with("main.rs") // Direct main.rs
        //     && !file_path.starts_with(project_root.join("src").join("bin")) // Files under src/bin/
        //     && file_path != &project_root.join("build.rs");
        let is_lib_context = is_library_file(file_path, project_root);


        // Skip build.rs for some checks like unwrap/expect, as they are common there
        if file_path.ends_with("build.rs") {
            // Could have specific checks for build.rs if desired
            // continue; // Or let specific checks decide
        }


        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                findings.push(Finding::new(
                    "IO001", // File Read Error
                    format!("Failed to read file {:?}: {}", file_path, e),
                    Severity::Warning,
                    Some(file_path.to_string_lossy().into_owned()),
                ));
                continue;
            }
        };

        // Iterate over lines to get line numbers for findings
        for (line_num, line_content) in content.lines().enumerate() {
            let line_number_for_finding = line_num + 1; // 1-indexed

            // Check for .unwrap() in library context
            if is_lib_context && UNWRAP_REGEX.is_match(line_content) && !file_path.ends_with("build.rs") {
                findings.push(Finding::new(
                    "CODE001",
                    format!("'.unwrap()' used in library context. Consider using '?' or pattern matching."),
                    Severity::Warning,
                    Some(file_path.to_string_lossy().into_owned()),
                ).with_line(line_number_for_finding));
            }

            // Check for .expect() in library context
            if is_lib_context && EXPECT_REGEX.is_match(line_content) && !file_path.ends_with("build.rs") {
                findings.push(Finding::new(
                    "CODE002",
                    format!("'.expect()' used in library context. While better than unwrap, prefer '?' or specific error handling."),
                    Severity::Note, // expect is slightly better than unwrap
                    Some(file_path.to_string_lossy().into_owned()),
                ).with_line(line_number_for_finding));
            }

            // Check for println!/dbg! in library context
            if is_lib_context && PRINTLN_DBG_REGEX.is_match(line_content) && !file_path.ends_with("build.rs") {
                 // Further refine: allow in main fn of examples, benches.
                 // This check is tricky without knowing the exact role of the file.
                 // For now, broad check on `is_lib_context`.
                findings.push(Finding::new(
                    "CODE003",
                    format!("Diagnostic macro (println! or dbg!) found in library context. Remove before release."),
                    Severity::Note,
                    Some(file_path.to_string_lossy().into_owned()),
                ).with_line(line_number_for_finding));
            }

            // Check for TODO/FIXME comments (applies to all files)
            if TODO_COMMENT_REGEX.is_match(line_content) {
                let comment_type = TODO_COMMENT_REGEX.captures(line_content).unwrap().get(1).unwrap().as_str();
                findings.push(Finding::new(
                    "CODE004",
                    format!("Found '{}' comment. Address or create an issue for it.", comment_type),
                    Severity::Note,
                    Some(file_path.to_string_lossy().into_owned()),
                ).with_line(line_number_for_finding));
            }
        }
    }
    findings
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
    let is_likely_library = manifest_data.map_or(false, |m| {
        // Check if [lib] path is specified, or if name is specified (implies lib)
        // This part of `toml` parsing for manifest.lib might need to be added to `CargoManifest` struct
        m.package.as_ref().map_or(false, |p| {
            // A common pattern is that the package name is used for the lib if not specified
            // This is still a heuristic. cargo_metadata is better.
            let default_lib_name = p.name.replace('-', "_");
            project_root.join("src").join(format!("{}.rs", default_lib_name)).exists() || has_lib_rs
        })
    });


    if let Some(pkg) = manifest_data.and_then(|m| m.package.as_ref()) {

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
                .map_or(false, |r_val| {
                    // if r_val is a bool and false, then it's fine
                    if let Some(b) = r_val.as_bool() { // Assuming readme can be bool or string
                        b == false
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
                .map_or(false, |p| {
                    // Assuming you add `license_file: Option<String>` to your Package struct
                    // p.license_file.as_ref().map_or(false, |lf| project_root.join(lf).is_file())
                    // For now, let's assume it's not specified if Package struct doesn't have it
                    p.license.is_some() && p.license.as_deref() != Some("") // If license is specified, a file is good practice
                });

            if !license_file_specified || manifest_data.and_then(|m| m.package.as_ref()).map_or(true, |p| p.license.is_none()) {
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