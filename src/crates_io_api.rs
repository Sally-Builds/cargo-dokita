use serde::Deserialize;
use reqwest::blocking::Client; // If using blocking client
use std::time::Duration;


const CRATES_IO_API_BASE: &str = "https://crates.io/api/v1/crates";
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[derive(Deserialize, Debug)]
pub struct CratesIoCrate {
    #[serde(rename = "crate")] // The main data is under a "crate" key
    crate_data: CrateData,
    versions: Vec<CrateVersion>, // List of all versions, useful but we mainly need newest
}

#[derive(Deserialize, Debug)]
struct CrateData {
    max_version: String, // The newest version string (stable)
}

// Not strictly needed if we only use max_version, but good for completeness
#[derive(Deserialize, Debug)]
struct CrateVersion {
    num: String, // Version number string
    yanked: bool,
}


pub fn get_latest_versions_from_crates_io(crate_name: &str, client: &Client) -> Result<String, String> {
    let url = format!("{}/{}", CRATES_IO_API_BASE, crate_name);

    let res = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .timeout(Duration::from_secs(30))
        .send()
        .map_err(|e| format!("Failed to send request to crate.io for {}: {}", crate_name, e));

    let api_response: CratesIoCrate = match res {
        Ok(res) => {
            if !res.status().is_success() {
                return Err(
                    format!("crates.io API request for {} failed with status: {}",crate_name, res.status()))
            }

            res.json().map_err(|e| format!("Failed to parse JSON response from crates.io for {}: {}", crate_name, e))?
        },
        Err(e) => {
            return Err(
                format!("Something went wrong - {}", e)
            )
        }
    };

    Ok(api_response.crate_data.max_version)
}

// First, add this testable version of your function to your main code:
pub fn get_latest_versions_from_crates_io_with_base_url(
    crate_name: &str, 
    client: &reqwest::blocking::Client, 
    base_url: &str
) -> Result<String, String> {
    let url = format!("{}/{}", base_url, crate_name);

    let res = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .timeout(Duration::from_secs(30))
        .send()
        .map_err(|e| format!("Failed to send request to crate.io for {}: {}", crate_name, e));

    let api_response: CratesIoCrate = match res {
        Ok(res) => {
            if !res.status().is_success() {
                return Err(
                    format!("crates.io API request for {} failed with status: {}",crate_name, res.status()))
            }

            res.json().map_err(|e| format!("Failed to parse JSON response from crates.io for {}: {}", crate_name, e))?
        },
        Err(e) => {
            return Err(
                format!("Something went wrong - {}", e)
            )
        }
    };

    Ok(api_response.crate_data.max_version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use reqwest::blocking::Client;
    use serde_json::json;
    use std::time::Duration;

    // Helper function to create a test client
    fn create_test_client() -> Client {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create test client")
    }

    // Test data helpers
    fn create_mock_crates_io_response() -> serde_json::Value {
        json!({
            "crate": {
                "max_version": "1.2.3"
            },
            "versions": [
                {
                    "num": "1.2.3",
                    "yanked": false
                },
                {
                    "num": "1.2.2",
                    "yanked": false
                },
                {
                    "num": "1.2.1",
                    "yanked": true
                }
            ]
        })
    }

    fn create_mock_crates_io_response_with_prerelease() -> serde_json::Value {
        json!({
            "crate": {
                "max_version": "2.0.0-beta.1"
            },
            "versions": [
                {
                    "num": "2.0.0-beta.1",
                    "yanked": false
                },
                {
                    "num": "1.9.9",
                    "yanked": false
                }
            ]
        })
    }

    #[test]
    fn test_successful_api_call() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/serde");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(create_mock_crates_io_response());
        });

        let client = create_test_client();
        let result = get_latest_versions_from_crates_io_with_base_url("serde", &client, &server.base_url());

        mock.assert();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1.2.3");
    }

    #[test]
    fn test_successful_api_call_with_prerelease() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/tokio");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(create_mock_crates_io_response_with_prerelease());
        });

        let client = create_test_client();
        let result = get_latest_versions_from_crates_io_with_base_url("tokio", &client, &server.base_url());

        mock.assert();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2.0.0-beta.1");
    }

    #[test]
    fn test_api_returns_404() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/nonexistent-crate");
            then.status(404)
                .body("Not Found");
        });

        let client = create_test_client();
        let result = get_latest_versions_from_crates_io_with_base_url("nonexistent-crate", &client, &server.base_url());

        mock.assert();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("crates.io API request for nonexistent-crate failed with status: 404"));
    }

    #[test]
    fn test_api_returns_500() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/some-crate");
            then.status(500)
                .body("Internal Server Error");
        });

        let client = create_test_client();
        let result = get_latest_versions_from_crates_io_with_base_url("some-crate", &client, &server.base_url());

        mock.assert();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("crates.io API request for some-crate failed with status: 500"));
    }

    #[test]
    fn test_invalid_json_response() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/invalid-json-crate");
            then.status(200)
                .header("content-type", "application/json")
                .body("{ invalid json }");
        });

        let client = create_test_client();
        let result = get_latest_versions_from_crates_io_with_base_url("invalid-json-crate", &client, &server.base_url());

        mock.assert();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Failed to parse JSON response from crates.io for invalid-json-crate"));
    }

    #[test]
    fn test_missing_crate_field_in_response() {
        let server = MockServer::start();
        let invalid_response = json!({
            "versions": [
                {
                    "num": "1.0.0",
                    "yanked": false
                }
            ]
        });

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/missing-field-crate");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(invalid_response);
        });

        let client = create_test_client();
        let result = get_latest_versions_from_crates_io_with_base_url("missing-field-crate", &client, &server.base_url());

        mock.assert();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Failed to parse JSON response from crates.io for missing-field-crate"));
    }

    #[test]
    fn test_correct_user_agent_header() {
        let server = MockServer::start();
        let expected_user_agent = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/user-agent-test")
                .header("User-Agent", expected_user_agent);
            then.status(200)
                .header("content-type", "application/json")
                .json_body(create_mock_crates_io_response());
        });

        let client = create_test_client();
        let result = get_latest_versions_from_crates_io_with_base_url("user-agent-test", &client, &server.base_url());

        mock.assert();
        assert!(result.is_ok());
    }

    #[test]
    fn test_crate_name_with_special_characters() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/my-special_crate.name");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(create_mock_crates_io_response());
        });

        let client = create_test_client();
        let result = get_latest_versions_from_crates_io_with_base_url("my-special_crate.name", &client, &server.base_url());

        mock.assert();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1.2.3");
    }

    // Test struct deserialization directly (no HTTP needed)
    #[test]
    fn test_crates_io_crate_deserialization() {
        let json_data = create_mock_crates_io_response();
        let json_string = serde_json::to_string(&json_data).unwrap();
        let parsed: Result<CratesIoCrate, _> = serde_json::from_str(&json_string);
        
        assert!(parsed.is_ok());
        let crate_info = parsed.unwrap();
        assert_eq!(crate_info.crate_data.max_version, "1.2.3");
        assert_eq!(crate_info.versions.len(), 3);
        assert_eq!(crate_info.versions[0].num, "1.2.3");
        assert!(!crate_info.versions[0].yanked);
        assert!(crate_info.versions[2].yanked);
    }

    #[test]
    fn test_crate_version_deserialization() {
        let json_data = r#"{"num": "1.0.0", "yanked": true}"#;
        let parsed: Result<CrateVersion, _> = serde_json::from_str(json_data);
        
        assert!(parsed.is_ok());
        let version = parsed.unwrap();
        assert_eq!(version.num, "1.0.0");
        assert!(version.yanked);
    }

    #[test]
    fn test_crate_data_deserialization() {
        let json_data = r#"{"max_version": "2.1.0"}"#;
        let parsed: Result<CrateData, _> = serde_json::from_str(json_data);
        
        assert!(parsed.is_ok());
        let crate_data = parsed.unwrap();
        assert_eq!(crate_data.max_version, "2.1.0");
    }

    #[test]
    fn test_constants() {
        assert_eq!(CRATES_IO_API_BASE, "https://crates.io/api/v1/crates");
        assert!(USER_AGENT.contains(env!("CARGO_PKG_NAME")));
        assert!(USER_AGENT.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_user_agent_format() {
        let user_agent = USER_AGENT;
        assert!(user_agent.contains("/"));
        let parts: Vec<&str> = user_agent.split('/').collect();
        assert_eq!(parts.len(), 2);
        assert!(!parts[0].is_empty()); // package name
        assert!(!parts[1].is_empty()); // version
    }

    #[test]
    fn test_url_construction_logic() {
        let base = "http://example.com";
        let crate_name = "serde";
        let expected_url = format!("{}/{}", base, crate_name);
        
        assert_eq!(expected_url, "http://example.com/serde");
    }
}

// Alternative: Simple tests that work without any HTTP mocking
// Use these if you prefer minimal dependencies
#[cfg(test)]
mod simple_unit_tests {
    use super::*;

    #[test]
    fn test_crates_io_response_parsing() {
        let json_str = r#"{
            "crate": {
                "max_version": "1.5.0"
            },
            "versions": [
                {
                    "num": "1.5.0",
                    "yanked": false
                },
                {
                    "num": "1.4.9",
                    "yanked": true
                }
            ]
        }"#;

        let parsed: CratesIoCrate = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed.crate_data.max_version, "1.5.0");
        assert_eq!(parsed.versions.len(), 2);
        assert_eq!(parsed.versions[0].num, "1.5.0");
        assert!(!parsed.versions[0].yanked);
        assert!(parsed.versions[1].yanked);
    }

    #[test]
    fn test_error_cases_json_parsing() {
        // Test missing crate field
        let invalid_json = r#"{
            "versions": [
                {
                    "num": "1.0.0",
                    "yanked": false
                }
            ]
        }"#;
        
        let result: Result<CratesIoCrate, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());

        // Test missing max_version field
        let invalid_json2 = r#"{
            "crate": {},
            "versions": []
        }"#;
        
        let result2: Result<CratesIoCrate, _> = serde_json::from_str(invalid_json2);
        assert!(result2.is_err());

        // Test completely invalid JSON
        let invalid_json3 = "{ this is not valid json }";
        let result3: Result<CratesIoCrate, _> = serde_json::from_str(invalid_json3);
        assert!(result3.is_err());
    }

    #[test]
    fn test_version_struct_variations() {
        // Test normal version
        let version_json = r#"{"num": "2.0.0", "yanked": false}"#;
        let version: CrateVersion = serde_json::from_str(version_json).unwrap();
        assert_eq!(version.num, "2.0.0");
        assert!(!version.yanked);

        // Test yanked version
        let yanked_json = r#"{"num": "1.9.9", "yanked": true}"#;
        let yanked: CrateVersion = serde_json::from_str(yanked_json).unwrap();
        assert_eq!(yanked.num, "1.9.9");
        assert!(yanked.yanked);

        // Test prerelease version
        let prerelease_json = r#"{"num": "3.0.0-alpha.1", "yanked": false}"#;
        let prerelease: CrateVersion = serde_json::from_str(prerelease_json).unwrap();
        assert_eq!(prerelease.num, "3.0.0-alpha.1");
        assert!(!prerelease.yanked);
    }

    #[test]
    fn test_crate_data_struct_variations() {
        // Test stable version
        let stable_json = r#"{"max_version": "3.1.4"}"#;
        let stable: CrateData = serde_json::from_str(stable_json).unwrap();
        assert_eq!(stable.max_version, "3.1.4");

        // Test prerelease version
        let prerelease_json = r#"{"max_version": "4.0.0-rc.2"}"#;
        let prerelease: CrateData = serde_json::from_str(prerelease_json).unwrap();
        assert_eq!(prerelease.max_version, "4.0.0-rc.2");

        // Test version with patch
        let patch_json = r#"{"max_version": "2.1.15"}"#;
        let patch: CrateData = serde_json::from_str(patch_json).unwrap();
        assert_eq!(patch.max_version, "2.1.15");
    }

    #[test]
    fn test_real_world_response_structure() {
        // This mimics a real crates.io response structure
        let real_response = r#"{
            "crate": {
                "max_version": "1.0.210"
            },
            "versions": [
                {
                    "num": "1.0.210",
                    "yanked": false
                },
                {
                    "num": "1.0.209",
                    "yanked": false
                },
                {
                    "num": "1.0.208",
                    "yanked": true
                }
            ]
        }"#;

        let parsed: CratesIoCrate = serde_json::from_str(real_response).unwrap();
        assert_eq!(parsed.crate_data.max_version, "1.0.210");
        assert_eq!(parsed.versions.len(), 3);
        
        // Verify the first version matches max_version
        assert_eq!(parsed.versions[0].num, parsed.crate_data.max_version);
        assert!(!parsed.versions[0].yanked);
        
        // Verify yanked version
        assert!(parsed.versions[2].yanked);
    }
}

/*
Dependencies for your Cargo.toml:

For full HTTP mocking tests:
[dev-dependencies]
httpmock = "0.7"
serde_json = "1.0"

For simple unit tests only:
[dev-dependencies]
serde_json = "1.0"

Note: httpmock works with blocking HTTP clients, unlike wiremock which is async-focused.
*/