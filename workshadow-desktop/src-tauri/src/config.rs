use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::capture::CaptureConfig;
use crate::ocr::OcrConfig;
use crate::privacy::PrivacyConfig;
use crate::search::SearchConfig;
use crate::storage::StorageConfig;

/// Top-level application configuration, mirroring ~/.workshadow/config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub capture: CaptureConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub ocr: OcrConfig,
    #[serde(default)]
    pub privacy: PrivacyConfig,
    #[serde(default)]
    pub search: SearchConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            capture: CaptureConfig::default(),
            storage: StorageConfig::default(),
            ocr: OcrConfig::default(),
            privacy: PrivacyConfig::default(),
            search: SearchConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load config from the default path (~/.workshadow/config.toml).
    pub fn load() -> Self {
        let config_path = Self::default_config_path();
        if config_path.exists() {
            Self::load_from(&config_path).unwrap_or_default()
        } else {
            let config = Self::default();
            config.save().ok();
            config
        }
    }

    /// Load config from a specific path.
    pub fn load_from(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save config to the default path.
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::default_config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    fn default_config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".workshadow")
            .join("config.toml")
    }

    /// Get the resolved data directory (expanding ~).
    pub fn data_dir(&self) -> PathBuf {
        let path = &self.storage.data_dir;
        if path.starts_with("~/") {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(&path[2..])
        } else {
            PathBuf::from(path)
        }
    }

    /// Get the resolved search index directory.
    pub fn index_dir(&self) -> PathBuf {
        let path = &self.search.index_dir;
        if path.starts_with("~/") {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(&path[2..])
        } else {
            PathBuf::from(path)
        }
    }
}
