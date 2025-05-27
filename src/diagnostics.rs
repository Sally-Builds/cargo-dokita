use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}