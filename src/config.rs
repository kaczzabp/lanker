use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub device_name: String,
    pub port: u16,
    pub discovery_port: u16,
    pub download_dir: PathBuf,
    pub chunk_size: usize,
    pub max_concurrent_transfers: usize,
}

impl Default for Config {
    fn default() -> Self {
        let device_name = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "Lanker-Device".into());

        let download_dir = directories::UserDirs::new()
            .and_then(|d| d.document_dir().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            device_name,
            port: 7878,
            discovery_port: 7879,
            download_dir,
            chunk_size: 16_777_216, // 16 MB
            max_concurrent_transfers: 4,
        }
    }
}

impl Config {
    fn config_path() -> PathBuf {
        let base = directories::ProjectDirs::from("com", "lanker", "Lanker")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("config.json")
    }

    /// Load config from disk, or create default if missing.
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match serde_json::from_str::<Config>(&contents) {
                    Ok(config) => {
                        tracing::info!("Config loaded from {}", path.display());
                        return config;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse config: {e}, using defaults");
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read config file: {e}, using defaults");
                }
            }
        }
        let config = Config::default();
        let _ = config.save();
        config
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("Serialize error: {e}"))?;
        std::fs::write(&path, json).map_err(|e| format!("Write error: {e}"))?;
        tracing::info!("Config saved to {}", path.display());
        Ok(())
    }
}
