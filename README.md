# Cargo Dokita

A comprehensive Rust project analysis tool that performs static analysis on Rust projects to identify potential issues, security vulnerabilities, and code quality problems.

## Overview

Cargo Dokita is a powerful linting and auditing tool for Rust projects that helps developers maintain high code quality, security, and best practices. It analyzes your Rust codebase and provides actionable feedback across multiple dimensions:

- **Dependency Management**: Identifies outdated dependencies and known security vulnerabilities
- **Code Quality**: Detects problematic patterns like excessive use of `.unwrap()` and debug statements in library code
- **Project Structure**: Validates project organization and checks for essential files (README, LICENSE)
- **Manifest Validation**: Ensures Cargo.toml contains proper metadata and follows best practices
- **Configurable Analysis**: Customizable rules through configuration files

## Installation

### From Source

```bash
git clone https://github.com/yourusername/cargo-dokita
cd cargo-dokita
cargo install --path .
```

### From crates.io (when published)

```bash
cargo install cargo-dokita
```

## Usage

### Basic Commands

Analyze the current directory:

```bash
cargo dokita
```

Analyze a specific project:

```bash
cargo dokita --project-path /path/to/project
```

Get JSON output for integration with other tools:

```bash
cargo dokita --format json
```

### Command Line Options

- `-p, --project-path <PATH>`: Specify the project path to analyze (default: current directory)
- `-f, --format <FORMAT>`: Output format - `human` (default) or `json`

### Examples

```bash
# Analyze current project with human-readable output
cargo dokita

# Analyze a different project
cargo dokita -p ../my-other-project

# Get machine-readable JSON output
cargo dokita --format json > analysis.json

# Analyze and save results
cargo dokita -p ./backend-service -f json | jq '.' > audit-report.json
```

## Configuration

Cargo Dokita supports configuration through a `.cargo-dokita.toml` file in your project root. This allows you to enable or disable specific checks according to your project needs.

### Configuration File Format

Create a `.cargo-dokita.toml` file in your project root:

```toml
[general]
# General configuration options (future use)

[checks]
enabled = { "MD001" = true, "MD002" = false, "CODE001" = true }
```

### Configuration Examples

Enable only specific metadata checks:

```toml
[checks]
enabled = {
    "MD001" = true,  # Description check
    "MD002" = false, # License check (disabled)
    "MD003" = true,  # Repository check
    "MD004" = true   # README check
}
```

Disable code pattern checks for prototypes:

```toml
[checks]
enabled = {
    "CODE001" = false, # Allow .unwrap() usage
    "CODE002" = false, # Allow .expect() usage
    "CODE003" = true,  # Still check for debug output
    "CODE004" = true   # Still check for TODO comments
}
```

Default behavior: If no configuration file is present, all checks are enabled by default.

## Checks

Cargo Dokita performs various types of analysis and assigns unique codes to each check. Here's a comprehensive list:

### Metadata Checks (MD)

| Code      | Severity     | Description                               | Fix                                                      |
| --------- | ------------ | ----------------------------------------- | -------------------------------------------------------- |
| **MD001** | Warning      | Missing 'description' field in Cargo.toml | Add a clear description of your package's purpose        |
| **MD002** | Warning      | Missing 'license' field in Cargo.toml     | Specify a license (e.g., "MIT", "Apache-2.0")            |
| **MD003** | Note         | Missing 'repository' field in Cargo.toml  | Add your repository URL for better discoverability       |
| **MD004** | Note/Warning | Missing or invalid 'readme' field         | Add a README file or set `readme = false` if intentional |
| **MD005** | Error        | Missing [package] section in Cargo.toml   | Add a proper [package] section with name and version     |

### Dependency Checks (DP)

| Code      | Severity | Description                                | Fix                                                 |
| --------- | -------- | ------------------------------------------ | --------------------------------------------------- |
| **DP001** | Warning  | Wildcard version "\*" used in dependencies | Specify explicit version ranges (e.g., "1.0")       |
| **DP002** | Warning  | Outdated dependency detected               | Update to the latest version available on crates.io |

### Code Quality Checks (CODE)

| Code        | Severity | Description                                       | Fix                                            |
| ----------- | -------- | ------------------------------------------------- | ---------------------------------------------- |
| **CODE001** | Warning  | `.unwrap()` used in library context               | Use `?` operator or proper error handling      |
| **CODE002** | Note     | `.expect()` used in library context               | Prefer `?` operator or specific error handling |
| **CODE003** | Note     | Debug macros (`println!`, `dbg!`) in library code | Remove debug output before release             |
| **CODE004** | Note     | TODO/FIXME/XXX comments found                     | Address or create issues for outstanding work  |

### Security/Audit Checks (SEC, AUD)

| Code       | Severity | Description                                | Fix                                              |
| ---------- | -------- | ------------------------------------------ | ------------------------------------------------ |
| **SEC001** | Error    | Known security vulnerability in dependency | Update to patched version or find alternative    |
| **AUD001** | Warning  | cargo-audit execution failed               | Install cargo-audit: `cargo install cargo-audit` |
| **AUD002** | Warning  | cargo-audit reported issues                | Review audit output and address findings         |
| **AUD003** | Warning  | Failed to parse cargo-audit output         | Check cargo-audit installation and output format |
| **AUD004** | Warning  | cargo-audit not found in PATH              | Install cargo-audit tool                         |

### Project Structure Checks (STRUCT)

| Code          | Severity | Description                                     | Fix                                                |
| ------------- | -------- | ----------------------------------------------- | -------------------------------------------------- |
| **STRUCT001** | Warning  | Missing main source files (lib.rs/main.rs/bin/) | Add proper source files or check project structure |
| **STRUCT002** | Note     | Missing README.md file                          | Create a README.md file documenting your project   |
| **STRUCT003** | Warning  | Missing LICENSE file                            | Add a LICENSE file (LICENSE, LICENSE-MIT, etc.)    |

### Lint Configuration Checks (LINT)

| Code        | Severity | Description                      | Fix                                                   |
| ----------- | -------- | -------------------------------- | ----------------------------------------------------- |
| **LINT001** | Note     | Missing recommended lint denials | Add `#![deny(warnings)]` to src/lib.rs or src/main.rs |

### API/Network Checks (API)

| Code       | Severity | Description                                   | Fix                                        |
| ---------- | -------- | --------------------------------------------- | ------------------------------------------ |
| **API001** | Warning  | Failed to fetch latest version from crates.io | Check network connection; may be temporary |

### I/O Checks (IO)

| Code      | Severity | Description                     | Fix                                  |
| --------- | -------- | ------------------------------- | ------------------------------------ |
| **IO001** | Warning  | File read error during analysis | Check file permissions and existence |

## Advanced Features

### Parallel Processing

Cargo Dokita leverages Rust's `rayon` crate for parallel processing of multiple files and checks, providing fast analysis even for large codebases.

### Integration Support

The JSON output format makes it easy to integrate Cargo Dokita into CI/CD pipelines:

```bash
# Exit with non-zero code if issues are found
cargo dokita --format json | jq '.[] | select(.severity == "Error")'
```

### Security Auditing

Cargo Dokita integrates with `cargo-audit` to check for known security vulnerabilities. Install it for complete security analysis:

```bash
cargo install cargo-audit
```

## Contributing

We welcome contributions! Here are ways you can help:

### Code Style

- Follow standard Rust formatting (`cargo fmt`)
- Ensure all tests pass (`cargo test`)
- Add tests for new functionality
- Use meaningful commit messages

### Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes and add tests
4. Run the test suite (`cargo test`)
5. Run formatting (`cargo fmt`)
6. Run clippy (`cargo clippy`)
7. Submit a pull request

### Adding New Checks

To add a new check:

1. Define the check logic in the appropriate module (`src/code_checks.rs`, `src/manifest.rs`, etc.)
2. Add a unique error code following the existing pattern
3. Add comprehensive tests
4. Update this README with the new check documentation
5. Consider configurability through the config system

### Reporting Issues

- Use the GitHub issue tracker
- Provide clear reproduction steps
- Include relevant project structure and configuration
- Specify cargo-dokita version and Rust version

## License

This project is licensed under [Your License Here] - see the LICENSE file for details.

## Acknowledgments

- Built with the Rust ecosystem: `cargo_metadata`, `clap`, `rayon`, `regex`, `serde`, and more
- Inspired by tools like `cargo-audit`, `clippy`, and other Rust quality tools
- Thanks to the Rust community for feedback and contributions
