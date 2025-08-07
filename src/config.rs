use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

use crate::types::{FanMode, PerformanceMode};

const CONFIG_DIR: &str = "/etc/omenix";
const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub fan_mode: FanMode,
    pub performance_mode: PerformanceMode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fan_mode: FanMode::Bios,
            performance_mode: PerformanceMode::Balanced,
        }
    }
}

impl Config {
    /// Get the path to the config file
    fn config_file_path() -> PathBuf {
        Path::new(CONFIG_DIR).join(CONFIG_FILE)
    }

    /// Load configuration from file, creating default if it doesn't exist
    pub fn load() -> Self {
        let config_path = Self::config_file_path();
        
        match fs::read_to_string(&config_path) {
            Ok(content) => {
                match serde_json::from_str::<Config>(&content) {
                    Ok(config) => {
                        info!("Loaded configuration from {:?}: {:?}", config_path, config);
                        config
                    }
                    Err(e) => {
                        warn!("Failed to parse config file {:?}: {}. Using defaults.", config_path, e);
                        Self::default()
                    }
                }
            }
            Err(e) => {
                debug!("Config file {:?} not found: {}. Using defaults.", config_path, e);
                Self::default()
            }
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_path = Self::config_file_path();
        
        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        fs::write(&config_path, content)?;
        info!("Saved configuration to {:?}: {:?}", config_path, self);
        Ok(())
    }

    /// Update fan mode and save to file
    pub fn update_fan_mode(&mut self, mode: FanMode) -> Result<(), std::io::Error> {
        self.fan_mode = mode;
        self.save()
    }

    /// Update performance mode and save to file
    pub fn update_performance_mode(&mut self, mode: PerformanceMode) -> Result<(), std::io::Error> {
        self.performance_mode = mode;
        self.save()
    }
}