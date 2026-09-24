//! Built-in [`ModelCatalog`]: static per-provider tables plus provider-local caches
//! (`~/.codex/models_cache.json`). User overrides are layered on top by the runtime.

use agtrace_types::{ModelCatalog, Provider};

use crate::claude::models as claude_models;
use crate::codex::models as codex_models;
use crate::codex::models_cache::CodexModelsCache;

/// Model catalog backed by built-in tables and provider caches. Never has user overrides.
#[derive(Debug, Clone, Default)]
pub struct BuiltinModelCatalog {
    codex_cache: Option<CodexModelsCache>,
}

impl BuiltinModelCatalog {
    /// Tables only (no provider caches). Deterministic; used by tests and the demo.
    pub fn tables_only() -> Self {
        Self::default()
    }

    /// Tables plus provider caches read from their default locations.
    pub fn load_default() -> Self {
        Self {
            codex_cache: CodexModelsCache::load_default(),
        }
    }

    /// Tables plus an explicit Codex models cache.
    pub fn with_codex_cache(cache: Option<CodexModelsCache>) -> Self {
        Self { codex_cache: cache }
    }
}

impl ModelCatalog for BuiltinModelCatalog {
    fn user_override(&self, _provider: Provider, _model: Option<&str>) -> Option<u64> {
        None
    }

    fn provider_cache(&self, provider: Provider, model: &str) -> Option<u64> {
        match provider {
            Provider::Codex => self.codex_cache.as_ref()?.effective_context_window(model),
            Provider::ClaudeCode => None,
        }
    }

    fn table(&self, provider: Provider, model: &str) -> Option<u64> {
        match provider {
            Provider::ClaudeCode => claude_models::context_window(model),
            Provider::Codex => codex_models::context_window(model),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_provider_scoped() {
        let cat = BuiltinModelCatalog::tables_only();
        assert_eq!(
            cat.table(Provider::ClaudeCode, "claude-opus-5-5"),
            Some(1_000_000)
        );
        assert_eq!(cat.table(Provider::Codex, "claude-opus-5-5"), None);
        assert_eq!(cat.table(Provider::Codex, "gpt-6-astra"), Some(258_400));
        assert_eq!(cat.provider_cache(Provider::Codex, "gpt-6-astra"), None);
    }

    #[test]
    fn codex_cache_is_consulted() {
        let cache = CodexModelsCache::parse(
            r#"{"models":[{"slug":"gpt-7","context_window":500000,"effective_context_window_percent":90}]}"#,
        );
        let cat = BuiltinModelCatalog::with_codex_cache(cache);
        assert_eq!(cat.provider_cache(Provider::Codex, "gpt-7"), Some(450_000));
        assert_eq!(cat.provider_cache(Provider::ClaudeCode, "gpt-7"), None);
    }
}
