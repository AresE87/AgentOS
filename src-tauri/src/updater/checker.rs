use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use tauri_plugin_updater::UpdaterExt;

const DEFAULT_GITHUB_REPO: &str = "AresE87/AgentOS";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
    pub checked_at: String,
    pub updater_configured: bool,
    pub install_supported: bool,
    pub status_mode: String,
    pub check_strategy: String,
    pub release_url: String,
    pub manifest_url: String,
    pub status_message: Option<String>,
}

pub struct UpdateChecker;

impl UpdateChecker {
    /// Check GitHub releases for a newer version.
    /// This is the honest fallback when signed updater artifacts are not configured.
    pub async fn check_for_update(current_version: &str, repo: &str) -> Result<UpdateInfo, String> {
        let repo = normalize_repo(repo);
        let client = reqwest::Client::new();
        let url = format!("https://api.github.com/repos/{repo}/releases/latest");

        let response = client
            .get(&url)
            .header("User-Agent", "AgentOS-Updater")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("Update check failed: {e}"))?;

        let checked_at = chrono::Utc::now().to_rfc3339();
        let release_url = release_page_url(repo);
        let manifest_url = latest_manifest_url(repo);

        if !response.status().is_success() {
            return Ok(UpdateInfo {
                current_version: current_version.to_string(),
                latest_version: None,
                update_available: false,
                release_notes: None,
                download_url: Some(release_url.clone()),
                checked_at,
                updater_configured: false,
                install_supported: false,
                status_mode: operational_mode(false, false).to_string(),
                check_strategy: "github_release_api".to_string(),
                release_url,
                manifest_url,
                status_message: Some(format!(
                    "GitHub release check returned status {}.",
                    response.status()
                )),
            });
        }

        let release: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let tag = release
            .get("tag_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim_start_matches('v')
            .to_string();
        let notes = release
            .get("body")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let html_url = release
            .get("html_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| release_url.clone());
        let latest_version = if tag.is_empty() {
            None
        } else {
            Some(tag.clone())
        };
        let update_available = latest_version
            .as_deref()
            .map(|latest| version_is_newer(latest, current_version))
            .unwrap_or(false);

        Ok(UpdateInfo {
            current_version: current_version.to_string(),
            latest_version,
            update_available,
            release_notes: notes,
            download_url: Some(html_url),
            checked_at,
            updater_configured: false,
            install_supported: false,
            status_mode: operational_mode(false, false).to_string(),
            check_strategy: "github_release_api".to_string(),
            release_url,
            manifest_url,
            status_message: None,
        })
    }

    /// Check updates using the official Tauri updater when the signed manifest is configured.
    /// Falls back to GitHub Releases metadata for check-only visibility when the signed flow
    /// cannot be validated locally.
    pub async fn check_for_update_with_app<R: Runtime>(
        app: &AppHandle<R>,
        current_version: &str,
        repo: &str,
        updater_pubkey: &str,
    ) -> Result<UpdateInfo, String> {
        let repo = normalize_repo(repo);
        let pubkey = updater_pubkey.trim();
        let mut info = Self::check_for_update(current_version, repo).await?;
        info.updater_configured = updater_is_configured(repo, pubkey);
        info.status_mode = operational_mode(info.updater_configured, info.install_supported).into();

        if !info.updater_configured {
            info.status_message =
                Some("Signed updater install is disabled: missing updater public key.".to_string());
            return Ok(info);
        }

        let updater = build_tauri_updater(app, repo, pubkey)?;
        match updater.check().await {
            Ok(Some(update)) => {
                info.latest_version = Some(update.version.clone());
                info.update_available = true;
                info.release_notes = update.body.clone().or(info.release_notes);
                info.download_url = Some(update.download_url.to_string());
                info.install_supported = true;
                info.status_mode =
                    operational_mode(info.updater_configured, info.install_supported).to_string();
                info.check_strategy = "tauri_updater".to_string();
                info.status_message = None;
            }
            Ok(None) => {
                info.update_available = false;
                info.install_supported = true;
                info.status_mode =
                    operational_mode(info.updater_configured, info.install_supported).to_string();
                info.check_strategy = "tauri_updater".to_string();
                if info.latest_version.is_none() {
                    info.latest_version = Some(current_version.to_string());
                }
                info.status_message = None;
            }
            Err(e) => {
                info.install_supported = false;
                info.status_mode =
                    operational_mode(info.updater_configured, info.install_supported).to_string();
                info.status_message = Some(format!(
                    "Signed updater is configured, but latest.json could not be used: {e}"
                ));
            }
        }

        Ok(info)
    }

    /// Download and install the currently available signed update.
    pub async fn install_update<R: Runtime>(
        app: &AppHandle<R>,
        repo: &str,
        updater_pubkey: &str,
    ) -> Result<(), String> {
        let repo = normalize_repo(repo);
        let pubkey = updater_pubkey.trim();

        if !updater_is_configured(repo, pubkey) {
            return Err(
                "Signed updater install is disabled: missing updater public key.".to_string(),
            );
        }

        let updater = build_tauri_updater(app, repo, pubkey)?;
        let update = updater
            .check()
            .await
            .map_err(|e| format!("Updater check failed: {e}"))?;
        let Some(update) = update else {
            return Err("No update available from the configured updater manifest.".to_string());
        };

        let version = update.version.clone();
        update
            .download_and_install(|_, _| {}, || {})
            .await
            .map_err(|e| format!("Failed to download and install update {version}: {e}"))?;

        Ok(())
    }
}

fn build_tauri_updater<R: Runtime>(
    app: &AppHandle<R>,
    repo: &str,
    updater_pubkey: &str,
) -> Result<tauri_plugin_updater::Updater, String> {
    let endpoint = reqwest::Url::parse(&latest_manifest_url(repo))
        .map_err(|e| format!("Invalid updater endpoint: {e}"))?;
    app.updater_builder()
        .pubkey(updater_pubkey.to_string())
        .endpoints(vec![endpoint])
        .map_err(|e| format!("Invalid updater configuration: {e}"))?
        .build()
        .map_err(|e| format!("Failed to build updater: {e}"))
}

fn normalize_repo(repo: &str) -> &str {
    let repo = repo.trim();
    if repo.is_empty() {
        DEFAULT_GITHUB_REPO
    } else {
        repo
    }
}

pub fn latest_manifest_url(repo: &str) -> String {
    let repo = normalize_repo(repo);
    format!("https://github.com/{repo}/releases/latest/download/latest.json")
}

pub fn release_page_url(repo: &str) -> String {
    let repo = normalize_repo(repo);
    format!("https://github.com/{repo}/releases")
}

pub fn updater_is_configured(repo: &str, updater_pubkey: &str) -> bool {
    !normalize_repo(repo).trim().is_empty() && !updater_pubkey.trim().is_empty()
}

pub fn operational_mode(updater_configured: bool, install_supported: bool) -> &'static str {
    if install_supported {
        "install_ready"
    } else if updater_configured {
        "manifest_pending"
    } else {
        "check_only"
    }
}

/// Compare semver strings: returns true if `latest` > `current`.
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

    #[test]
    fn latest_manifest_url_points_to_github_latest_json() {
        assert_eq!(
            latest_manifest_url("AresE87/AgentOS"),
            "https://github.com/AresE87/AgentOS/releases/latest/download/latest.json"
        );
    }

    #[test]
    fn latest_manifest_url_points_to_expected_location() {
        assert_eq!(
            latest_manifest_url("AresE87/AgentOS"),
            "https://github.com/AresE87/AgentOS/releases/latest/download/latest.json"
        );
    }

    #[test]
    fn updater_requires_pubkey_for_install_support() {
        assert!(updater_is_configured("AresE87/AgentOS", "pubkey"));
        assert!(!updater_is_configured("AresE87/AgentOS", ""));
    }

    #[test]
    fn operational_mode_distinguishes_check_only_manifest_pending_and_install_ready() {
        assert_eq!(operational_mode(false, false), "check_only");
        assert_eq!(operational_mode(true, false), "manifest_pending");
        assert_eq!(operational_mode(true, true), "install_ready");
    }
}
