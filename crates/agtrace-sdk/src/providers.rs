//! Lightweight provider operations without database.
//!
//! The [`Providers`] type provides access to provider operations (parsing, diagnostics)
//! without requiring a full workspace with database. Use this when you only need to:
//!
//! - Parse log files directly
//! - Run diagnostics on providers
//! - Check file parseability
//! - Inspect file contents
//!
//! For session querying and indexing, use [`Client`](crate::Client) instead.
//!
//! # Examples
//!
//! ```no_run
//! use agtrace_sdk::Providers;
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Auto-detect providers from system paths
//! let providers = Providers::detect()?;
//!
//! // Parse a log file
//! let events = providers.parse_auto(Path::new("/path/to/log.jsonl"))?;
//! println!("Parsed {} events", events.len());
//!
//! // Run diagnostics
//! let results = providers.diagnose()?;
//! for result in &results {
//!     println!("{}: {} files, {} successful",
//!         result.provider_name,
//!         result.total_files,
//!         result.successful);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Custom Provider Configuration
//!
//! ```no_run
//! use agtrace_sdk::Providers;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let providers = Providers::builder()
//!     .provider("claude_code", "/custom/.claude/projects")
//!     .provider("codex", "/custom/.codex/sessions")
//!     .build()?;
//! # Ok(())
//! # }
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::types::{
    AgentEvent, CheckResult, Config, DiagnoseResult, InspectResult, ProviderConfig,
};

/// Lightweight client for provider operations without database.
///
/// This type provides access to provider-level operations (parsing, diagnostics)
/// without requiring a full workspace with database indexing.
///
/// # When to use `Providers` vs `Client`
///
/// | Operation | `Providers` | `Client` |
/// |-----------|-------------|----------|
/// | Parse log files | Yes | Yes (via `.providers()`) |
/// | Run diagnostics | Yes | Yes (via `.system()`) |
/// | Check/inspect files | Yes | Yes (via `.system()`) |
/// | List sessions | No | Yes |
/// | Query sessions | No | Yes |
/// | Watch events | No | Yes |
/// | Index operations | No | Yes |
///
/// Use `Providers` for:
/// - Quick file parsing without workspace setup
/// - Diagnostics on provider log directories
/// - CI/CD validation of log files
/// - Tools that only need read-only file access
///
/// Use `Client` for:
/// - Session browsing and querying
/// - Real-time event monitoring
/// - Full workspace operations
#[derive(Clone)]
pub struct Providers {
    config: Arc<Config>,
    /// (provider_name, log_root)
    provider_configs: Vec<(String, PathBuf)>,
}

impl Providers {
    /// Create with auto-detected providers from system paths.
    ///
    /// Scans default log directories for each supported provider
    /// (Claude Code, Codex, Gemini) and enables those that exist.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use agtrace_sdk::Providers;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let providers = Providers::detect()?;
    /// println!("Detected {} providers", providers.list().len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn detect() -> Result<Self> {
        let config = Config::detect_providers().map_err(Error::Runtime)?;
        Ok(Self::with_config(config))
    }

    /// Create with explicit configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use agtrace_sdk::{Providers, types::Config};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config::detect_providers()?;
    /// let providers = Providers::with_config(config);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_config(config: Config) -> Self {
        let provider_configs: Vec<(String, PathBuf)> = config
            .enabled_providers()
            .into_iter()
            .map(|(name, cfg)| (name.clone(), cfg.log_root.clone()))
            .collect();

        Self {
            config: Arc::new(config),
            provider_configs,
        }
    }

    /// Create a builder for fine-grained configuration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use agtrace_sdk::Providers;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let providers = Providers::builder()
    ///     .provider("claude_code", "/custom/.claude/projects")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> ProvidersBuilder {
        ProvidersBuilder::new()
    }

    // =========================================================================
    // Operations
    // =========================================================================

    /// Parse a log file with auto-detected provider.
    ///
    /// Automatically detects the appropriate provider based on file path
    /// and parses it into events.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use agtrace_sdk::Providers;
    /// use std::path::Path;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let providers = Providers::detect()?;
    /// let events = providers.parse_auto(Path::new("/path/to/session.jsonl"))?;
    /// for event in &events {
    ///     println!("{:?}", event.payload);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn parse_auto(&self, path: &Path) -> Result<Vec<AgentEvent>> {
        let path_str = path
            .to_str()
            .ok_or_else(|| Error::InvalidInput("Path contains invalid UTF-8".to_string()))?;

        let adapter = agtrace_providers::detect_adapter_from_path(path_str)
            .map_err(|_| Error::NotFound("No suitable provider detected for file".to_string()))?;

        adapter
            .parser
            .parse_file(path)
            .map_err(|e| Error::InvalidInput(format!("Parse error: {}", e)))
    }

    /// Parse a log file with a specific provider.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use agtrace_sdk::Providers;
    /// use std::path::Path;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let providers = Providers::detect()?;
    /// let events = providers.parse_file(Path::new("/path/to/log.jsonl"), "claude_code")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn parse_file(&self, path: &Path, provider_name: &str) -> Result<Vec<AgentEvent>> {
        let adapter = agtrace_providers::create_adapter(provider_name)
            .map_err(|_| Error::NotFound(format!("Unknown provider: {}", provider_name)))?;

        adapter
            .parser
            .parse_file(path)
            .map_err(|e| Error::InvalidInput(format!("Parse error: {}", e)))
    }

    /// Run diagnostics on all configured providers.
    ///
    /// Scans each provider's log directory and reports parsing statistics.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use agtrace_sdk::Providers;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let providers = Providers::detect()?;
    /// let results = providers.diagnose()?;
    ///
    /// for result in &results {
    ///     let success_rate = if result.total_files > 0 {
    ///         (result.successful as f64 / result.total_files as f64) * 100.0
    ///     } else {
    ///         100.0
    ///     };
    ///     println!("{}: {:.1}% success ({}/{})",
    ///         result.provider_name,
    ///         success_rate,
    ///         result.successful,
    ///         result.total_files);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn diagnose(&self) -> Result<Vec<DiagnoseResult>> {
        let providers: Vec<_> = self
            .provider_configs
            .iter()
            .filter_map(|(name, path)| {
                agtrace_providers::create_adapter(name)
                    .ok()
                    .map(|adapter| (adapter, path.clone()))
            })
            .collect();

        agtrace_runtime::DoctorService::diagnose_all(&providers).map_err(Error::Runtime)
    }

    /// Check if a file can be parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use agtrace_sdk::Providers;
    /// use std::path::Path;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let providers = Providers::detect()?;
    /// let result = providers.check_file(Path::new("/path/to/log.jsonl"), None)?;
    /// println!("Status: {:?}", result.status);
    /// # Ok(())
    /// # }
    /// ```
    pub fn check_file(&self, path: &Path, provider: Option<&str>) -> Result<CheckResult> {
        let path_str = path
            .to_str()
            .ok_or_else(|| Error::InvalidInput("Path contains invalid UTF-8".to_string()))?;

        let (adapter, provider_name) = if let Some(name) = provider {
            let adapter = agtrace_providers::create_adapter(name)
                .map_err(|_| Error::NotFound(format!("Provider: {}", name)))?;
            (adapter, name.to_string())
        } else {
            let adapter = agtrace_providers::detect_adapter_from_path(path_str)
                .map_err(|_| Error::NotFound("No suitable provider detected".to_string()))?;
            let name = format!("{} (auto-detected)", adapter.id());
            (adapter, name)
        };

        agtrace_runtime::DoctorService::check_file(path_str, &adapter, &provider_name)
            .map_err(Error::Runtime)
    }

    /// Inspect raw file contents.
    ///
    /// Returns the first N lines of the file, optionally parsed as JSON.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use agtrace_sdk::Providers;
    /// use std::path::Path;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let result = Providers::inspect_file(Path::new("/path/to/log.jsonl"), 10, true)?;
    /// println!("Showing {} of {} lines", result.shown_lines, result.total_lines);
    /// # Ok(())
    /// # }
    /// ```
    pub fn inspect_file(path: &Path, lines: usize, json_format: bool) -> Result<InspectResult> {
        let path_str = path
            .to_str()
            .ok_or_else(|| Error::InvalidInput("Path contains invalid UTF-8".to_string()))?;

        agtrace_runtime::DoctorService::inspect_file(path_str, lines, json_format)
            .map_err(Error::Runtime)
    }

    /// List configured providers.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use agtrace_sdk::Providers;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let providers = Providers::detect()?;
    /// for (name, config) in providers.list() {
    ///     println!("{}: {:?}", name, config.log_root);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn list(&self) -> Vec<(&String, &ProviderConfig)> {
        self.config.enabled_providers()
    }

    /// Get current configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get provider configurations as (name, log_root) pairs.
    pub fn provider_configs(&self) -> &[(String, PathBuf)] {
        &self.provider_configs
    }
}

// =============================================================================
// ProvidersBuilder
// =============================================================================

/// Builder for configuring [`Providers`].
///
/// Allows programmatic configuration of providers without relying on
/// filesystem detection or TOML files.
///
/// # Examples
///
/// ```no_run
/// use agtrace_sdk::Providers;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Start from auto-detected providers and add custom ones
/// let providers = Providers::builder()
///     .auto_detect()
///     .provider("claude_code", "/custom/claude/path")
///     .build()?;
///
/// // Or configure entirely manually
/// let providers = Providers::builder()
///     .provider("claude_code", "/path/to/.claude/projects")
///     .provider("codex", "/path/to/.codex/sessions")
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct ProvidersBuilder {
    config: Option<Config>,
    providers: Vec<(String, PathBuf)>,
}

impl ProvidersBuilder {
    /// Create a new builder with no providers configured.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from a TOML file.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use agtrace_sdk::Providers;
    /// use std::path::Path;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let providers = Providers::builder()
    ///     .config_file(Path::new("/path/to/config.toml"))?
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn config_file(mut self, path: impl AsRef<Path>) -> Result<Self> {
        let config = Config::load_from(&path.as_ref().to_path_buf()).map_err(Error::Runtime)?;
        self.config = Some(config);
        Ok(self)
    }

    /// Use explicit configuration.
    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Add a provider with custom log root.
    ///
    /// This overrides any provider with the same name from config file
    /// or auto-detection.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use agtrace_sdk::Providers;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let providers = Providers::builder()
    ///     .provider("claude_code", "/custom/.claude/projects")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn provider(mut self, name: &str, log_root: impl Into<PathBuf>) -> Self {
        self.providers.push((name.to_string(), log_root.into()));
        self
    }

    /// Enable auto-detection of providers.
    ///
    /// Scans default log directories for each supported provider
    /// and enables those that exist.
    pub fn auto_detect(mut self) -> Self {
        match Config::detect_providers() {
            Ok(config) => {
                self.config = Some(config);
            }
            Err(_) => {
                // Silently ignore detection errors
            }
        }
        self
    }

    /// Build the `Providers` instance.
    pub fn build(self) -> Result<Providers> {
        let mut config = self.config.unwrap_or_default();

        // Apply manual provider overrides
        for (name, log_root) in self.providers {
            config.set_provider(
                name,
                ProviderConfig {
                    enabled: true,
                    log_root,
                    context_window_override: None,
                },
            );
        }

        Ok(Providers::with_config(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creates_empty_providers() {
        let providers = Providers::builder().build().unwrap();
        assert!(providers.list().is_empty());
    }

    #[test]
    fn test_builder_with_manual_provider() {
        let providers = Providers::builder()
            .provider("claude_code", "/tmp/test")
            .build()
            .unwrap();

        assert_eq!(providers.list().len(), 1);
        let (name, config) = &providers.list()[0];
        assert_eq!(*name, "claude_code");
        assert_eq!(config.log_root, PathBuf::from("/tmp/test"));
    }

    #[test]
    fn test_with_config() {
        let mut config = Config::default();
        config.set_provider(
            "test_provider".to_string(),
            ProviderConfig {
                enabled: true,
                log_root: PathBuf::from("/test/path"),
                context_window_override: None,
            },
        );

        let providers = Providers::with_config(config);
        assert_eq!(providers.list().len(), 1);
    }
}
