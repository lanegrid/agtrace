use crate::config::Config;
use agtrace_index::Database;
use agtrace_providers::{create_provider, LogProvider};
use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};

pub struct ExecutionContext {
    data_dir: PathBuf,
    db: OnceCell<Database>,
    config: OnceCell<Config>,
    pub project_root: Option<PathBuf>,
    pub all_projects: bool,
}

impl ExecutionContext {
    pub fn new(
        data_dir: PathBuf,
        project_root: Option<String>,
        all_projects: bool,
    ) -> Result<Self> {
        let project_root = project_root
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok());

        Ok(Self {
            data_dir,
            project_root,
            all_projects,
            db: OnceCell::new(),
            config: OnceCell::new(),
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn db(&self) -> Result<&Database> {
        self.db.get_or_try_init(|| {
            let db_path = self.data_dir.join("agtrace.db");
            Database::open(&db_path)
        })
    }

    pub fn config(&self) -> Result<&Config> {
        self.config.get_or_try_init(|| {
            let config_path = self.data_dir.join("config.toml");
            Config::load_from(&config_path)
        })
    }

    pub fn resolve_provider(&self, provider_name: &str) -> Result<(Box<dyn LogProvider>, PathBuf)> {
        let config = self.config()?;
        let provider_config = config
            .providers
            .get(provider_name)
            .ok_or_else(|| anyhow!("Provider '{}' not found in config", provider_name))?;

        if !provider_config.enabled {
            anyhow::bail!("Provider '{}' is not enabled", provider_name);
        }

        let provider = create_provider(provider_name)?;
        Ok((provider, provider_config.log_root.clone()))
    }

    pub fn default_provider(&self) -> Result<String> {
        let config = self.config()?;
        config
            .providers
            .iter()
            .find(|(_, p)| p.enabled)
            .map(|(name, _)| name.clone())
            .ok_or_else(|| {
                anyhow!("No enabled provider found. Run 'agtrace init' to configure providers.")
            })
    }

    pub fn resolve_providers(
        &self,
        provider_filter: &str,
    ) -> Result<Vec<(Box<dyn LogProvider>, PathBuf)>> {
        use agtrace_providers::create_all_providers;

        if provider_filter == "all" {
            let config = self.config()?;
            let all_providers = create_all_providers();
            let mut result = Vec::new();

            for provider in all_providers {
                let provider_name = provider.name();
                if let Some(provider_config) = config.providers.get(provider_name) {
                    if provider_config.enabled {
                        result.push((provider, provider_config.log_root.clone()));
                    }
                }
            }

            if result.is_empty() {
                anyhow::bail!(
                    "No enabled providers found. Run 'agtrace init' to configure providers."
                );
            }

            Ok(result)
        } else {
            let (provider, log_root) = self.resolve_provider(provider_filter)?;
            Ok(vec![(provider, log_root)])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_context() -> (TempDir, ExecutionContext) {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        // Create a minimal config file for testing
        let config_path = data_dir.join("config.toml");
        let config_content = r#"
[providers.claude_code]
enabled = true
log_root = "/tmp/claude_logs"

[providers.codex]
enabled = true
log_root = "/tmp/codex_logs"

[providers.gemini]
enabled = false
log_root = "/tmp/gemini_logs"
"#;
        fs::write(&config_path, config_content).unwrap();

        let ctx = ExecutionContext::new(data_dir, None, false).unwrap();
        (temp_dir, ctx)
    }

    #[test]
    fn test_execution_context_lazy_loading() {
        let (_temp_dir, ctx) = setup_test_context();

        // Verify OnceCell fields are initially empty
        assert!(ctx.db.get().is_none(), "DB should not be loaded initially");
        assert!(
            ctx.config.get().is_none(),
            "Config should not be loaded initially"
        );

        // Access config - should load now
        let config_result = ctx.config();
        assert!(config_result.is_ok(), "Config should load successfully");
        assert!(
            ctx.config.get().is_some(),
            "Config should be loaded after access"
        );

        // DB should still not be loaded
        assert!(
            ctx.db.get().is_none(),
            "DB should remain unloaded until accessed"
        );
    }

    #[test]
    fn test_resolve_providers_filtering_all() {
        let (_temp_dir, ctx) = setup_test_context();

        let providers = ctx.resolve_providers("all").unwrap();

        // Should return only enabled providers (claude_code and codex)
        assert_eq!(providers.len(), 2, "Should return 2 enabled providers");

        // Verify provider names
        let provider_names: Vec<&str> = providers.iter().map(|(p, _)| p.name()).collect();
        assert!(
            provider_names.contains(&"claude_code"),
            "Should include claude_code"
        );
        assert!(
            provider_names.contains(&"codex"),
            "Should include codex"
        );
        assert!(
            !provider_names.contains(&"gemini"),
            "Should not include disabled gemini"
        );
    }

    #[test]
    fn test_resolve_providers_filtering_specific() {
        let (_temp_dir, ctx) = setup_test_context();

        let providers = ctx.resolve_providers("claude_code").unwrap();

        assert_eq!(
            providers.len(),
            1,
            "Should return exactly 1 provider for specific filter"
        );
        assert_eq!(
            providers[0].0.name(),
            "claude_code",
            "Should return the requested provider"
        );
    }

    #[test]
    fn test_resolve_provider_disabled() {
        let (_temp_dir, ctx) = setup_test_context();

        let result = ctx.resolve_provider("gemini");

        assert!(
            result.is_err(),
            "Should fail to resolve disabled provider"
        );
        let err_msg = result.err().unwrap().to_string();
        assert!(
            err_msg.contains("not enabled"),
            "Error should mention provider is not enabled: {}",
            err_msg
        );
    }

    #[test]
    fn test_default_provider() {
        let (_temp_dir, ctx) = setup_test_context();

        let default = ctx.default_provider().unwrap();

        // Should return first enabled provider (order may vary based on HashMap)
        assert!(
            default == "claude_code" || default == "codex",
            "Default provider should be one of the enabled providers"
        );
    }

    #[test]
    fn test_data_dir_access() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();
        let ctx = ExecutionContext::new(data_dir.clone(), None, false).unwrap();

        assert_eq!(
            ctx.data_dir(),
            data_dir.as_path(),
            "data_dir() should return correct path"
        );
    }

    #[test]
    fn test_project_root_normalization() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        // Test with explicit project root
        let ctx_with_root =
            ExecutionContext::new(data_dir.clone(), Some("/test/path".to_string()), false)
                .unwrap();
        assert_eq!(
            ctx_with_root.project_root,
            Some(PathBuf::from("/test/path")),
            "Should use explicit project root"
        );

        // Test with None - should fall back to current dir
        let ctx_without_root = ExecutionContext::new(data_dir, None, false).unwrap();
        assert!(
            ctx_without_root.project_root.is_some(),
            "Should fall back to current directory"
        );
    }

    #[test]
    fn test_database_path_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        // Create config for ExecutionContext
        let config_path = data_dir.join("config.toml");
        fs::write(&config_path, "[providers]").unwrap();

        let ctx = ExecutionContext::new(data_dir.clone(), None, false).unwrap();

        // Access the db - this will create "agtrace.db"
        let _db_result = ctx.db();

        // Verify that "agtrace.db" was created (not "db.sqlite")
        let expected_db_path = data_dir.join("agtrace.db");
        let wrong_db_path = data_dir.join("db.sqlite");

        assert!(
            expected_db_path.exists(),
            "Should create agtrace.db, not db.sqlite"
        );
        assert!(
            !wrong_db_path.exists(),
            "Should NOT create db.sqlite (old bug path)"
        );
    }
}
