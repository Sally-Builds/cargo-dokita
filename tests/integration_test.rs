mod test_helpers;
use test_helpers::*;

#[test]
fn test_analyze_valid_rust_project() {
    let env = TestEnvironment::new();

    // Create a perfect project that should pass all checks
    create_perfect_project(env.path(), "test-project").unwrap();

    let result = analyze_project_text(env.path());
    assert!(
        result.is_ok(),
        "Expected project to pass all checks, but got: {:?}",
        result
    );
}

#[test]
fn test_analyze_project_with_issues() {
    let env = TestEnvironment::new();

    // Create a basic project that will have issues (missing metadata, etc.)
    ProjectBuilder::new("test-project")
        .dependency("serde", "1.0")
        .build_in(env.path())
        .unwrap();

    let result = analyze_project_text(env.path());
    // This should have issues (missing README, LICENSE, description, etc.)
    assert!(result.is_err(), "Expected project to have issues");

    // We can also check what issues were found
    let findings = analyze_project_expect_issues(env.path()).unwrap();
    assert!(!findings.is_empty(), "Should have found some issues");
}

#[test]
fn test_analyze_non_rust_project() {
    let env = TestEnvironment::new();

    create_non_rust_project(env.path()).unwrap();

    let result = analyze_project_text(env.path());
    assert!(matches!(result, Err(cargo_dokita::MyError::NotRustProject)));
}

#[test]
fn test_analyze_nonexistent_path() {
    let result = cargo_dokita::analyze_project("/nonexistent/path", "text");
    assert!(matches!(
        result,
        Err(cargo_dokita::MyError::UnresolvableProjectPath)
    ));
}

#[test]
fn test_analyze_with_json_output() {
    let env = TestEnvironment::new();

    // Create a perfect project that should pass all checks
    create_perfect_project(env.path(), "test-project").unwrap();

    let result = analyze_project_json(env.path());
    assert!(result.is_ok());
}

#[test]
fn test_analyze_project_with_missing_metadata() {
    let env = TestEnvironment::new();

    // Create a basic project with minimal metadata (will have issues)
    ProjectBuilder::new("test-project")
        .build_in(env.path())
        .unwrap();

    let result = analyze_project_text(env.path());
    // Should have issues due to missing metadata
    assert!(result.is_err());
}

#[test]
fn test_analyze_project_with_unsafe_code() {
    let env = TestEnvironment::new();

    ProjectBuilder::new("test-project")
        .build_in(env.path())
        .unwrap();

    create_unsafe_main(env.path()).unwrap();

    let findings = analyze_project_expect_issues(env.path()).unwrap();
    // Should find some issues, but unsafe code itself isn't flagged by current checks
    // It will have issues due to missing metadata
    assert!(!findings.is_empty());
}

#[test]
fn test_analyze_project_with_config_file() {
    let env = TestEnvironment::new();

    // Create a basic project
    ProjectBuilder::new("test-project")
        .build_in(env.path())
        .unwrap();

    // Create dokita config that disables some checks
    create_permissive_dokita_config(env.path()).unwrap();

    // Even with permissive config, may still have some issues
    let _findings = analyze_project_expect_issues(env.path()).unwrap();
    // The config should reduce the number of findings, but we'll still have some
    // (like missing files that config can't fix)
}

#[test]
fn test_analyze_workspace_project() {
    let env = TestEnvironment::new();

    // Create workspace project using test helpers
    create_workspace_project(env.path(), &["member1", "member2"]).unwrap();

    let _findings = analyze_project_expect_issues(env.path()).unwrap();
    // Workspace projects typically have fewer metadata requirements
    // but may still have structural issues
}
