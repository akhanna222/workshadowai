pub mod encryption;
pub mod exclusions;
pub mod audit;
pub mod keymanager;

use serde::{Deserialize, Serialize};

/// Privacy-related configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub excluded_apps: Vec<String>,
    pub excluded_url_patterns: Vec<String>,
    pub recording_indicator: bool,
    pub global_hotkey_pause: String,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            excluded_apps: vec![
                "1Password".to_string(),
                "Bitwarden".to_string(),
                "LastPass".to_string(),
                "KeePass".to_string(),
            ],
            excluded_url_patterns: vec![
                "*bank*".to_string(),
                "*paypal*".to_string(),
            ],
            recording_indicator: true,
            global_hotkey_pause: "CmdOrCtrl+Shift+P".to_string(),
        }
    }
}
