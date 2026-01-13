// Configuration Storage Service
// Handles config file read/write and version backup

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub version: String,
    pub default_provider: Option<String>,
    pub proxy: Option<ProxyConfig>,
    pub detection: DetectionConfig,
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub api_keys: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub enabled: bool,
    pub http: Option<String>,
    pub https: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionConfig {
    #[serde(default = "default_sensitivity")]
    pub sensitivity: String,
    #[serde(default = "default_true")]
    pub use_perplexity: bool,
    #[serde(default = "default_true")]
    pub use_stylometry: bool,
    #[serde(default = "default_chunk_size")]
    pub chunk_size_tokens: i32,
    #[serde(default = "default_overlap")]
    pub overlap_tokens: i32,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            sensitivity: "medium".to_string(),
            use_perplexity: true,
            use_stylometry: true,
            chunk_size_tokens: 1500,
            overlap_tokens: 150,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub enabled: bool,
    pub model: Option<String>,
    pub base_url: Option<String>,
}

fn default_sensitivity() -> String { "medium".to_string() }
fn default_true() -> bool { true }
fn default_chunk_size() -> i32 { 1500 }
fn default_overlap() -> i32 { 150 }

pub struct ConfigStore {
    config_dir: PathBuf,
    config_file: PathBuf,
}

impl ConfigStore {
    pub fn new(config_dir: PathBuf) -> Self {
        let config_file = config_dir.join("config.json");
        Self { config_dir, config_file }
    }

    /// Get default config directory
    pub fn default_config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("cheekAI"))
    }

    /// Ensure config directory exists
    pub fn ensure_dir(&self) -> Result<(), String> {
        fs::create_dir_all(&self.config_dir)
            .map_err(|e| format!("Failed to create config dir: {}", e))
    }

    /// Load configuration from file
    pub fn load(&self) -> Result<AppConfig, String> {
        if !self.config_file.exists() {
            return Ok(AppConfig::default());
        }

        let content = fs::read_to_string(&self.config_file)
            .map_err(|e| format!("Failed to read config: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config: {}", e))
    }

    /// Save configuration to file
    pub fn save(&self, config: &AppConfig) -> Result<(), String> {
        self.ensure_dir()?;

        // Create backup if file exists
        if self.config_file.exists() {
            self.create_backup()?;
        }

        let content = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(&self.config_file, content)
            .map_err(|e| format!("Failed to write config: {}", e))
    }

    /// Create a backup of current config
    fn create_backup(&self) -> Result<(), String> {
        let backup_dir = self.config_dir.join("backups");
        fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("Failed to create backup dir: {}", e))?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_file = backup_dir.join(format!("config_{}.json", timestamp));

        fs::copy(&self.config_file, &backup_file)
            .map_err(|e| format!("Failed to create backup: {}", e))?;

        // Keep only last 10 backups
        self.cleanup_old_backups(&backup_dir, 10)?;

        Ok(())
    }

    /// Remove old backups, keeping only the most recent N
    fn cleanup_old_backups(&self, backup_dir: &PathBuf, keep: usize) -> Result<(), String> {
        let mut entries: Vec<_> = fs::read_dir(backup_dir)
            .map_err(|e| format!("Failed to read backup dir: {}", e))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
            .collect();

        if entries.len() <= keep {
            return Ok(());
        }

        // Sort by modification time (oldest first)
        entries.sort_by_key(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });

        // Remove oldest entries
        for entry in entries.iter().take(entries.len() - keep) {
            let _ = fs::remove_file(entry.path());
        }

        Ok(())
    }

    /// Get provider API key from config file
    pub fn get_api_key(&self, provider: &str) -> Result<Option<String>, String> {
        let config = self.load()?;
        Ok(config.api_keys.get(provider).cloned())
    }

    /// Store provider API key in config file
    pub fn set_api_key(&self, provider: &str, key: &str) -> Result<(), String> {
        let mut config = self.load()?;
        config.api_keys.insert(provider.to_string(), key.to_string());
        self.save(&config)
    }

    /// Delete provider API key from config file
    pub fn delete_api_key(&self, provider: &str) -> Result<(), String> {
        let mut config = self.load()?;
        config.api_keys.remove(provider);
        self.save(&config)
    }

    /// Get provider base URL from config file
    pub fn get_provider_url(&self, provider: &str) -> Result<Option<String>, String> {
        let config = self.load()?;
        Ok(config.providers.get(provider).and_then(|p| p.base_url.clone()))
    }

    /// Set provider base URL in config file
    pub fn set_provider_url(&self, provider: &str, url: &str) -> Result<(), String> {
        let mut config = self.load()?;
        let provider_config = config.providers.entry(provider.to_string()).or_default();
        provider_config.base_url = Some(url.to_string());
        self.save(&config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.detection.sensitivity, "medium");
        assert!(config.detection.use_perplexity);
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig {
            version: "1.0.0".to_string(),
            default_provider: Some("glm".to_string()),
            proxy: None,
            detection: DetectionConfig::default(),
            providers: HashMap::new(),
            api_keys: HashMap::new(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, "1.0.0");
    }
}
