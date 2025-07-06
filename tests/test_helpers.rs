use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Configuration for creating a test Rust project
#[derive(Default)]
pub struct ProjectBuilder {
    pub name: String,
    pub version: String,
    pub edition: String,
    pub dependencies: Vec<(String, String)>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub workspace_members: Vec<String>,
    pub is_workspace: bool,
}

impl ProjectBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            edition: "2021".to_string(),
            dependencies: Vec::new(),
            description: None,
            license: None,
            workspace_members: Vec::new(),
            is_workspace: false,
        }
    }

    pub fn version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    pub fn edition(mut self, edition: &str) -> Self {
        self.edition = edition.to_string();
        self
    }

    pub fn dependency(mut self, name: &str, version: &str) -> Self {
        self.dependencies
            .push((name.to_string(), version.to_string()));
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    #[allow(dead_code)]
    pub fn license(mut self, license: &str) -> Self {
        self.license = Some(license.to_string());
        self
    }

    pub fn workspace(mut self, members: &[&str]) -> Self {
        self.is_workspace = true;
        self.workspace_members = members.iter().map(|&s| s.to_string()).collect();
        self
    }

    pub fn build_in(self, project_path: &Path) -> std::io::Result<()> {
        if self.is_workspace {
            self.create_workspace(project_path)
        } else {
            self.create_package(project_path)
        }
    }

    fn create_package(&self, project_path: &Path) -> std::io::Result<()> {
        let mut cargo_toml = format!(
            r#"[package]
name = "{}"
version = "{}"
edition = "{}"
"#,
            self.name, self.version, self.edition
        );

        if let Some(ref description) = self.description {
            cargo_toml.push_str(&format!("description = \"{description}\"\n"));
        }

        if let Some(ref license) = self.license {
            cargo_toml.push_str(&format!("license = \"{license}\"\n"));
        }

        if !self.dependencies.is_empty() {
            cargo_toml.push_str("\n[dependencies]\n");
            for (name, version) in &self.dependencies {
                cargo_toml.push_str(&format!("{name} = \"{version}\"\n"));
            }
        }

        fs::write(project_path.join("Cargo.toml"), cargo_toml)?;

        // Create src directory
        fs::create_dir(project_path.join("src"))?;

        // Create main.rs or lib.rs based on project type
        if self.name.contains("lib") || self.name.contains("library") {
            fs::write(project_path.join("src/lib.rs"), "")?;
        } else {
            fs::write(project_path.join("src/main.rs"), "fn main() {}\n")?;
        }

        Ok(())
    }

    fn create_workspace(&self, project_path: &Path) -> std::io::Result<()> {
        let mut workspace_toml = "[workspace]\n".to_string();
        workspace_toml.push_str("members = [");
        for (i, member) in self.workspace_members.iter().enumerate() {
            if i > 0 {
                workspace_toml.push_str(", ");
            }
            workspace_toml.push_str(&format!("\"{member}\""));
        }
        workspace_toml.push_str("]\n");

        fs::write(project_path.join("Cargo.toml"), workspace_toml)?;

        // Create member projects
        for member in &self.workspace_members {
            let member_path = project_path.join(member);
            fs::create_dir(&member_path)?;

            ProjectBuilder::new(member)
                .edition(&self.edition)
                .build_in(&member_path)?;
        }

        Ok(())
    }
}

/// Helper to create a basic Rust project
#[allow(dead_code)]
pub fn create_basic_rust_project(project_path: &Path, name: &str) -> std::io::Result<()> {
    ProjectBuilder::new(name).build_in(project_path)
}

/// Helper to create a Rust project with dependencies
#[allow(dead_code)]
pub fn create_rust_project_with_deps(
    project_path: &Path,
    name: &str,
    dependencies: &[(&str, &str)],
) -> std::io::Result<()> {
    let mut builder = ProjectBuilder::new(name);
    for (dep_name, dep_version) in dependencies {
        builder = builder.dependency(dep_name, dep_version);
    }
    builder.build_in(project_path)
}

/// Helper to create a Rust project with missing metadata
#[allow(dead_code)]
pub fn create_rust_project_minimal(project_path: &Path, name: &str) -> std::io::Result<()> {
    ProjectBuilder::new(name).build_in(project_path)
}

/// Helper to create a Rust project with specific edition
#[allow(dead_code)]
pub fn create_rust_project_with_edition(
    project_path: &Path,
    name: &str,
    edition: &str,
) -> std::io::Result<()> {
    ProjectBuilder::new(name)
        .edition(edition)
        .build_in(project_path)
}

/// Helper to create source files with specific content
pub fn create_source_file(
    project_path: &Path,
    file_path: &str,
    content: &str,
) -> std::io::Result<()> {
    let full_path = project_path.join(file_path);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(full_path, content)
}

/// Helper to create unsafe code in main.rs
pub fn create_unsafe_main(project_path: &Path) -> std::io::Result<()> {
    let unsafe_code = r#"
fn main() {
    unsafe {
        let x = 5;
    }
}
"#;
    create_source_file(project_path, "src/main.rs", unsafe_code)
}

/// Helper to create a dokita config file
pub fn create_dokita_config(project_path: &Path, config_content: &str) -> std::io::Result<()> {
    fs::write(project_path.join("dokita.toml"), config_content)
}

/// Helper to create a standard dokita config that disables metadata requirements
pub fn create_permissive_dokita_config(project_path: &Path) -> std::io::Result<()> {
    let config = r#"
[metadata]
require_description = false
require_license = false
"#;
    create_dokita_config(project_path, config)
}

/// Helper to create a workspace project
pub fn create_workspace_project(project_path: &Path, members: &[&str]) -> std::io::Result<()> {
    ProjectBuilder::new("workspace")
        .workspace(members)
        .build_in(project_path)
}

/// Helper to create a non-Rust project (for negative testing)
pub fn create_non_rust_project(project_path: &Path) -> std::io::Result<()> {
    fs::write(project_path.join("README.md"), "Not a Rust project")
}

/// Test setup wrapper that provides a temporary directory and project path
pub struct TestEnvironment {
    #[allow(dead_code)]
    pub temp_dir: TempDir,
    pub project_path: PathBuf,
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl TestEnvironment {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_path_buf();
        Self {
            temp_dir,
            project_path,
        }
    }

    pub fn path(&self) -> &Path {
        &self.project_path
    }

    #[allow(dead_code)]
    pub fn path_str(&self) -> &str {
        self.project_path.to_str().unwrap()
    }
}

/// Macro to reduce boilerplate in test setup
#[macro_export]
macro_rules! test_env {
    () => {
        test_helpers::TestEnvironment::new()
    };
}

/// Analyze project helper that wraps the main function call
pub fn analyze_project_text(project_path: &Path) -> Result<(), cargo_dokita::MyError> {
    match cargo_dokita::analyze_project_for_test(project_path.to_str().unwrap(), "text") {
        Ok(findings) => {
            // For tests, we consider it successful if there are no errors/warnings
            if findings.iter().any(|f| {
                matches!(
                    f.severity,
                    cargo_dokita::diagnostics::Severity::Error
                        | cargo_dokita::diagnostics::Severity::Warning
                )
            }) {
                Err(cargo_dokita::MyError::HasIssues(findings))
            } else {
                Ok(())
            }
        }
        Err(e) => Err(e),
    }
}

/// Analyze project helper for JSON output
#[allow(dead_code)]
pub fn analyze_project_json(project_path: &Path) -> Result<(), cargo_dokita::MyError> {
    match cargo_dokita::analyze_project_for_test(project_path.to_str().unwrap(), "json") {
        Ok(findings) => {
            if findings.iter().any(|f| {
                matches!(
                    f.severity,
                    cargo_dokita::diagnostics::Severity::Error
                        | cargo_dokita::diagnostics::Severity::Warning
                )
            }) {
                Err(cargo_dokita::MyError::HasIssues(findings))
            } else {
                Ok(())
            }
        }
        Err(e) => Err(e),
    }
}

/// Helper to analyze project and expect it to find issues (for negative testing)
pub fn analyze_project_expect_issues(
    project_path: &Path,
) -> Result<Vec<cargo_dokita::diagnostics::Finding>, cargo_dokita::MyError> {
    cargo_dokita::analyze_project_for_test(project_path.to_str().unwrap(), "text")
}

/// Helper to create a project that should pass all checks
#[allow(dead_code)]
pub fn create_perfect_project(project_path: &Path, name: &str) -> std::io::Result<()> {
    // Create a project with all metadata and no issues
    ProjectBuilder::new(name)
        .description("A well-documented test project")
        .license("MIT")
        .build_in(project_path)?;

    // Create README
    let readme_content = format!(
        r#"# {name}

A well-documented test project.

## License

MIT
"#
    );
    std::fs::write(project_path.join("README.md"), readme_content)?;

    // Create LICENSE file
    std::fs::write(
        project_path.join("LICENSE"),
        "MIT License\n\nCopyright (c) 2024\n\nPermission is hereby granted...",
    )?;

    // Create a main.rs with proper linting
    let main_content = r#"#![deny(warnings)]

fn main() {
    println!("Hello, world!");
}
"#;
    std::fs::write(project_path.join("src/main.rs"), main_content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_builder() {
        let env = TestEnvironment::new();
        let result = ProjectBuilder::new("test-project")
            .version("0.2.0")
            .edition("2018")
            .dependency("serde", "1.0")
            .description("A test project")
            .build_in(env.path());

        assert!(result.is_ok());
        assert!(env.path().join("Cargo.toml").exists());
        assert!(env.path().join("src/main.rs").exists());
    }

    #[test]
    fn test_workspace_builder() {
        let env = TestEnvironment::new();
        let result = create_workspace_project(env.path(), &["member1", "member2"]);

        assert!(result.is_ok());
        assert!(env.path().join("Cargo.toml").exists());
        assert!(env.path().join("member1/Cargo.toml").exists());
        assert!(env.path().join("member2/Cargo.toml").exists());
    }
}
