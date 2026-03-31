use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
    pub checked_at: String,
}

pub struct UpdateChecker;

impl UpdateChecker {
    /// Check GitHub releases for a newer version
    pub async fn check_for_update(current_version: &str, repo: &str) -> Result<UpdateInfo, String> {
        let client = reqwest::Client::new();
        let url = format!("https://api.github.com/repos/{}/releases/latest", repo);

        let response = client
            .get(&url)
            .header("User-Agent", "AgentOS-Updater")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("Update check failed: {}", e))?;

        if !response.status().is_success() {
            return Ok(UpdateInfo {
                current_version: current_version.to_string(),
                latest_version: None,
                update_available: false,
                release_notes: None,
                download_url: None,
                checked_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        let release: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let tag = release
            .get("tag_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim_start_matches('v');
        let notes = release
            .get("body")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let html_url = release
            .get("html_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let update_available = version_is_newer(tag, current_version);

        Ok(UpdateInfo {
            current_version: current_version.to_string(),
            latest_version: Some(tag.to_string()),
            update_available,
            release_notes: notes,
            download_url: html_url,
            checked_at: chrono::Utc::now().to_rfc3339(),
        })
    }
}

/// Compare semver strings: returns true if `latest` > `current`
pub fn version_is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> { v.split('.').filter_map(|p| p.parse().ok()).collect() };
    let l = parse(latest);
    let c = parse(current);
    l > c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(version_is_newer("4.3.0", "4.2.0"));
        assert!(version_is_newer("5.0.0", "4.2.0"));
        assert!(version_is_newer("4.2.1", "4.2.0"));
        assert!(!version_is_newer("4.2.0", "4.2.0"));
        assert!(!version_is_newer("4.1.0", "4.2.0"));
        assert!(!version_is_newer("3.9.9", "4.2.0"));
    }
}
