use crate::{Error, Result};
use agtrace_types::{Provider, normalize_model_id};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// Resolve the workspace data directory path based on priority:
/// 1. Explicit path (with tilde expansion)
/// 2. AGTRACE_PATH environment variable (with tilde expansion)
/// 3. System data directory (recommended default)
/// 4. ~/.agtrace (fallback for systems without standard data directory)
pub fn resolve_workspace_path(explicit_path: Option<&str>) -> Result<PathBuf> {
    agtrace_core::resolve_workspace_path(explicit_path).map_err(|e| match e {
        agtrace_core::path::Error::Io(io_err) => Error::Io(io_err),
        agtrace_core::path::Error::Config(msg) => Error::Config(msg),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub log_root: PathBuf,
}

/// `[context_window]` section of `config.toml`: user overrides for the context window
/// resolver (design §3, layer 1).
///
/// ```toml
/// [context_window]
/// default_claude_code = 1000000   # provider default (`default_<provider>`)
/// "claude-opus-5" = 1000000       # longest-prefix match on the model id ([..] suffix stripped)
/// "gpt-5.6" = 258400
/// ```
///
/// A model-prefix entry wins over the provider default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContextWindowConfig {
    entries: BTreeMap<String, u64>,
}

const PROVIDER_DEFAULT_PREFIX: &str = "default_";

impl ContextWindowConfig {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Set a model-prefix override.
    pub fn set_model(&mut self, model_prefix: &str, tokens: u64) {
        self.entries.insert(model_prefix.to_string(), tokens);
    }

    /// Set a provider default override.
    pub fn set_provider_default(&mut self, provider: Provider, tokens: u64) {
        self.entries.insert(
            format!("{PROVIDER_DEFAULT_PREFIX}{}", provider.as_str()),
            tokens,
        );
    }

    /// Override for `provider` / `model`, if configured.
    pub fn lookup(&self, provider: Provider, model: Option<&str>) -> Option<u64> {
        let by_model = model.and_then(|m| {
            let key = normalize_model_id(m);
            self.entries
                .iter()
                .filter(|(k, _)| !k.starts_with(PROVIDER_DEFAULT_PREFIX))
                .map(|(k, v)| (normalize_model_id(k), *v))
                .filter(|(prefix, _)| !prefix.is_empty() && key.starts_with(prefix.as_str()))
                .max_by_key(|(prefix, _)| prefix.len())
                .map(|(_, v)| v)
        });
        by_model.or_else(|| {
            self.entries.iter().find_map(|(k, v)| {
                let name = k.strip_prefix(PROVIDER_DEFAULT_PREFIX)?;
                (Provider::from_name(name) == Some(provider)).then_some(*v)
            })
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default, skip_serializing_if = "ContextWindowConfig::is_empty")]
    pub context_window: ContextWindowConfig,
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
        let (config, ignored) = Self::parse(&content)?;
        for name in ignored {
            eprintln!(
                "Warning: ignoring unsupported provider '{}' in {}",
                name,
                path.display()
            );
        }
        Ok(config)
    }

    /// Parse config TOML, skipping `[providers.<name>]` sections for providers
    /// that are not supported (e.g. `gemini`, which was removed).
    ///
    /// Returns the config and the names of the ignored provider sections.
    fn parse(content: &str) -> Result<(Self, Vec<String>)> {
        let mut table: toml::Table = toml::from_str(content)?;
        let mut ignored = Vec::new();

        if let Some(toml::Value::Table(providers)) = table.get_mut("providers") {
            providers.retain(|name, _| {
                let keep = agtrace_providers::create_adapter(name).is_ok();
                if !keep {
                    ignored.push(name.to_string());
                }
                keep
            });
        }
        ignored.sort();

        let config: Config = table.try_into()?;
        Ok((config, ignored))
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
                    },
                );
            }
        }

        Ok(Config {
            providers,
            ..Config::default()
        })
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
    fn test_load_ignores_unknown_provider_sections() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.toml");
        std::fs::write(
            &config_path,
            r#"
[providers.claude_code]
enabled = true
log_root = "/home/user/.claude/projects"

[providers.gemini]
enabled = true
log_root = "/home/user/.gemini/tmp"
"#,
        )?;

        let config = Config::load_from(&config_path)?;
        assert_eq!(config.providers.len(), 1);
        assert!(config.providers.contains_key("claude_code"));
        assert!(!config.providers.contains_key("gemini"));

        let (_, ignored) = Config::parse(&std::fs::read_to_string(&config_path)?)?;
        assert_eq!(ignored, vec!["gemini".to_string()]);

        Ok(())
    }

    #[test]
    fn test_context_window_section() -> Result<()> {
        let (config, _) = Config::parse(
            r#"
[providers.claude_code]
enabled = true
log_root = "/x"
context_window_override = 123   # legacy key: ignored, not an error

[context_window]
default_claude_code = 200000
"claude-opus-5" = 1000000
"claude-opus-5-5" = 900000
"gpt-5.6" = 250000
"#,
        )?;
        let cw = &config.context_window;
        let cc = Provider::ClaudeCode;
        assert_eq!(cw.lookup(cc, Some("claude-opus-5-5[1m]")), Some(900_000));
        assert_eq!(cw.lookup(cc, Some("claude-opus-5")), Some(1_000_000));
        assert_eq!(cw.lookup(cc, Some("claude-haiku-4-5")), Some(200_000));
        assert_eq!(cw.lookup(cc, None), Some(200_000));
        assert_eq!(
            cw.lookup(Provider::Codex, Some("gpt-5.6-sol")),
            Some(250_000)
        );
        assert_eq!(cw.lookup(Provider::Codex, Some("gpt-6-astra")), None);
        assert_eq!(cw.lookup(Provider::Codex, None), None);
        Ok(())
    }

    #[test]
    fn test_context_window_round_trip() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("config.toml");
        let mut config = Config::default();
        config.context_window.set_model("gpt-5.6", 258_400);
        config
            .context_window
            .set_provider_default(Provider::ClaudeCode, 1_000_000);
        config.save_to(&path)?;
        let loaded = Config::load_from(&path)?;
        assert_eq!(loaded.context_window, config.context_window);

        // Empty section is not written.
        Config::default().save_to(&path)?;
        assert!(!std::fs::read_to_string(&path)?.contains("context_window"));
        Ok(())
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
