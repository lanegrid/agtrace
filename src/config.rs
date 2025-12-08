use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Provider configuration for a specific agent tool (Claude, Codex, Gemini, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Whether this provider is enabled for import
    pub enabled: bool,
    /// Root directory where this provider stores logs
    pub log_root: PathBuf,
}

/// Main configuration for agtrace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Provider-specific configurations
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

impl Config {
    /// Load config from the default location (~/.agtrace/config.toml)
    pub fn load() -> Result<Self> {
        let config_path = Self::default_path()?;
        Self::load_from(&config_path)
    }

    /// Load config from a specific path
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            // Return default config if file doesn't exist
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save config to the default location
    pub fn save(&self) -> Result<()> {
        let config_path = Self::default_path()?;
        self.save_to(&config_path)
    }

    /// Save config to a specific path
    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the default config file path (~/.agtrace/config.toml)
    pub fn default_path() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| anyhow::anyhow!("Could not determine home directory"))?;

        Ok(PathBuf::from(home).join(".agtrace").join("config.toml"))
    }

    /// Detect providers by checking default directories
    pub fn detect_providers() -> Result<Self> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| anyhow::anyhow!("Could not determine home directory"))?;

        let home_path = PathBuf::from(home);
        let mut providers = HashMap::new();

        // Check for Claude
        let claude_path = home_path.join(".claude").join("projects");
        if claude_path.exists() {
            providers.insert(
                "claude".to_string(),
                ProviderConfig {
                    enabled: true,
                    log_root: claude_path,
                },
            );
        }

        // Check for Codex
        let codex_path = home_path.join(".codex").join("sessions");
        if codex_path.exists() {
            providers.insert(
                "codex".to_string(),
                ProviderConfig {
                    enabled: true,
                    log_root: codex_path,
                },
            );
        }

        // Check for Gemini
        let gemini_path = home_path.join(".gemini").join("tmp");
        if gemini_path.exists() {
            providers.insert(
                "gemini".to_string(),
                ProviderConfig {
                    enabled: true,
                    log_root: gemini_path,
                },
            );
        }

        Ok(Config { providers })
    }

    /// Get enabled providers
    pub fn enabled_providers(&self) -> Vec<(&String, &ProviderConfig)> {
        self.providers
            .iter()
            .filter(|(_, config)| config.enabled)
            .collect()
    }

    /// Update or insert a provider configuration
    pub fn set_provider(&mut self, name: String, config: ProviderConfig) {
        self.providers.insert(name, config);
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            providers: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.providers.len(), 0);
    }

    #[test]
    fn test_config_save_and_load() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.toml");

        let mut config = Config::default();
        config.set_provider(
            "claude".to_string(),
            ProviderConfig {
                enabled: true,
                log_root: PathBuf::from("/home/user/.claude/projects"),
            },
        );

        config.save_to(&config_path)?;
        assert!(config_path.exists());

        let loaded = Config::load_from(&config_path)?;
        assert_eq!(loaded.providers.len(), 1);
        assert!(loaded.providers.contains_key("claude"));
        assert!(loaded.providers.get("claude").unwrap().enabled);

        Ok(())
    }

    #[test]
    fn test_enabled_providers() {
        let mut config = Config::default();
        config.set_provider(
            "claude".to_string(),
            ProviderConfig {
                enabled: true,
                log_root: PathBuf::from("/test/claude"),
            },
        );
        config.set_provider(
            "codex".to_string(),
            ProviderConfig {
                enabled: false,
                log_root: PathBuf::from("/test/codex"),
            },
        );

        let enabled = config.enabled_providers();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].0, "claude");
    }

    #[test]
    fn test_load_nonexistent_returns_default() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("nonexistent.toml");

        let config = Config::load_from(&config_path)?;
        assert_eq!(config.providers.len(), 0);

        Ok(())
    }
}
