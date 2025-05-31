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