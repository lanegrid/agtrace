use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// TODO: Path utilities are currently scattered across crates (technical debt).
// This function is temporarily in runtime/config.rs.
// Related utilities like `discover_project_root()` are in agtrace-types/src/util.rs
// but types should only contain schemas, not utilities.
// See: https://github.com/lanegrid/agtrace/issues/19

/// Resolve the workspace data directory path based on priority:
/// 1. Explicit path (with tilde expansion)
/// 2. AGTRACE_PATH environment variable (with tilde expansion)
/// 3. XDG data directory (recommended default)
/// 4. ~/.agtrace (fallback for systems without XDG)
pub fn resolve_workspace_path(explicit_path: Option<&str>) -> Result<PathBuf> {
    // Priority 1: Explicit path
    if let Some(path) = explicit_path {
        return Ok(expand_tilde(path));
    }

    // Priority 2: AGTRACE_PATH environment variable
    if let Ok(env_path) = std::env::var("AGTRACE_PATH") {
        return Ok(expand_tilde(&env_path));
    }

    // Priority 3: XDG data directory (recommended default)
    if let Some(data_dir) = dirs::data_dir() {
        return Ok(data_dir.join("agtrace"));
    }

    // Priority 4: Fallback to ~/.agtrace (last resort for systems without XDG)
    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home).join(".agtrace"));
    }

    // This should never happen, but provide a working directory fallback
    Err(Error::Config(
        "Could not determine workspace path: no HOME directory or XDG data directory found"
            .to_string(),
    ))
}

/// Expand tilde (~) in paths to the user's home directory
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(stripped);
    }
    PathBuf::from(path)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub log_root: PathBuf,
    #[serde(default)]
    pub context_window_override: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::default_path()?;
        Self::load_from(&config_path)
    }

    pub fn load_from(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::default_path()?;
        self.save_to(&config_path)
    }

    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn default_path() -> Result<PathBuf> {
        Ok(resolve_workspace_path(None)?.join("config.toml"))
    }

    pub fn detect_providers() -> Result<Self> {
        let mut providers = HashMap::new();

        for (name, path) in agtrace_providers::get_default_log_paths() {
            if path.exists() {
                providers.insert(
                    name,
                    ProviderConfig {
                        enabled: true,
                        log_root: path,
                        context_window_override: None,
                    },
                );
            }
        }

        Ok(Config { providers })
    }

    pub fn enabled_providers(&self) -> Vec<(&String, &ProviderConfig)> {
        self.providers
            .iter()
            .filter(|(_, config)| config.enabled)
            .collect()
    }

    pub fn set_provider(&mut self, name: String, config: ProviderConfig) {
        self.providers.insert(name, config);
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
                context_window_override: None,
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
                context_window_override: None,
            },
        );
        config.set_provider(
            "codex".to_string(),
            ProviderConfig {
                enabled: false,
                log_root: PathBuf::from("/test/codex"),
                context_window_override: None,
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
