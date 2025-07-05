//! # Cargo Dokita Diagnostics
//!
//! This module defines the core diagnostic structures used throughout Cargo Dokita
//! for reporting issues, warnings, and informational findings about Rust projects.
//!
//! ## Features
//! - **Severity Levels**: Categorizes findings as `Error`, `Warning`, or `Note`
//! - **Structured Findings**: Each finding includes a unique code, message, severity, and optional location information
//! - **Serialization Support**: All structures can be serialized/deserialized for output formatting or persistence
//! - **Builder Pattern**: Convenient methods for creating findings with optional line numbers
//!
//! ## Severity Levels
//! - **Error**: Critical issues that must be fixed (e.g., missing required fields)
//! - **Warning**: Issues that should be addressed (e.g., outdated dependencies)
//! - **Note**: Informational or best practice suggestions (e.g., style recommendations)
//!
//! ## Example Usage
//! ```rust
//! use cargo_dokita::diagnostics::{Finding, Severity};
//!
//! // Create a basic finding
//! let finding = Finding::new(
//!     "MD001",
//!     "Missing license field in Cargo.toml".to_string(),
//!     Severity::Error,
//!     Some("Cargo.toml".to_string())
//! );
//!
//! // Create a finding with line number
//! let finding_with_line = Finding::new(
//!     "MD002",
//!     "Consider updating this dependency".to_string(),
//!     Severity::Warning,
//!     Some("Cargo.toml".to_string())
//! ).with_line(15);
//! ```
//!
//! ## Finding Codes
//! Finding codes follow a pattern where:
//! - **MD**: Metadata-related issues (e.g., Cargo.toml problems)
//! - **ML**: Missing license or legal issues
//! - Additional prefixes may be added for different check categories

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Error,   // Must fix
    Warning, // Should fix
    Note,    // Informational / Best practice
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub code: String, // A unique code for the type of finding, e.g., "MD001" for missing license
    pub message: String,
    pub severity: Severity,
    pub file_path: Option<String>, // e.g., "Cargo.toml"
    pub line_number: Option<usize>, // Optional: for more precise location (harder for TOML)
}

impl Finding {
    pub fn new(code: &str, message: String, severity: Severity, file_path: Option<String>) -> Self {
        Finding {
            code: code.to_string(),
            message,
            severity,
            file_path,
            line_number: None, // Keep it simple for now
        }
    }
    pub fn with_line(mut self, line: usize) -> Self {
        self.line_number = Some(line);
        self
    }
}


#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn create_finding_with_file_path() {
        let code = "ML001";
        let message = String::from("No version specified in Cargo.toml");
        let severity = Severity::Error;
        let file_path = Some(String::from("/Cargo.toml"));

        let findings = Finding::new(code, message.clone(), severity, file_path.clone());

        assert_eq!(code, findings.code);
        assert_eq!(message, findings.message);
        assert_eq!(file_path, findings.file_path);
        assert_eq!(Severity::Error, findings.severity);

    }

    #[test]
    fn create_finding_without_file_path() {
        let finding = Finding::new("ML002", "Test message".to_string(), Severity::Warning, None);
        assert_eq!(None, finding.file_path);
    }
}