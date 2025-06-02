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
    // id: String, // The crate name
    max_version: String, // The newest version string (stable)
    // newest_version: String, // Could be a pre-release
    // Add more fields if needed, like description, documentation, repository
}

// Not strictly needed if we only use max_version, but good for completeness
#[derive(Deserialize, Debug)]
struct CrateVersion {
    num: String, // Version number string
    yanked: bool,
    // license: String,
    // created_at: String,
    // downloads: u64,
}


pub fn get_latest_versions_from_crates_io(crate_name: &str, client: &Client) -> Result<String, String> {
    let url = format!("{}/{}", CRATES_IO_API_BASE, crate_name);

    let res = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .timeout(Duration::from_secs(30))
        .send()
        .map_err(|e| format!("Failed to send request to crate.io for {}: {}", crate_name, e));

    // if res.unwrap().status().is_success() {

    // }
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