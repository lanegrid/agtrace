//! Model catalog used by the context window resolver: built-in tables and provider
//! caches (`agtrace-providers`) with the user's `config.toml` `[context_window]`
//! overrides layered on top.

use agtrace_providers::BuiltinModelCatalog;
use agtrace_types::{ModelCatalog, Provider};

use crate::config::ContextWindowConfig;

#[derive(Debug, Clone, Default)]
pub struct ConfiguredModelCatalog {
    overrides: ContextWindowConfig,
    builtin: BuiltinModelCatalog,
}

impl ConfiguredModelCatalog {
    pub fn new(overrides: ContextWindowConfig, builtin: BuiltinModelCatalog) -> Self {
        Self { overrides, builtin }
    }

    /// User overrides + built-in tables + provider caches from their default locations.
    pub fn load(overrides: ContextWindowConfig) -> Self {
        Self::new(overrides, BuiltinModelCatalog::load_default())
    }
}

impl ModelCatalog for ConfiguredModelCatalog {
    fn user_override(&self, provider: Provider, model: Option<&str>) -> Option<u64> {
        self.overrides.lookup(provider, model)
    }

    fn provider_cache(&self, provider: Provider, model: &str) -> Option<u64> {
        self.builtin.provider_cache(provider, model)
    }

    fn table(&self, provider: Provider, model: &str) -> Option<u64> {
        self.builtin.table(provider, model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_engine::{ContextEvidence, resolve_context_window};
    use agtrace_types::ContextSource;

    #[test]
    fn user_override_beats_everything() {
        let mut overrides = ContextWindowConfig::default();
        overrides.set_model("claude-opus-5", 200_000);
        let cat = ConfiguredModelCatalog::new(overrides, BuiltinModelCatalog::tables_only());
        let mut ev = ContextEvidence::with_model("claude-opus-5");
        ev.peak_context_tokens = 300_000;
        let w = resolve_context_window(Provider::ClaudeCode, &ev, &cat).unwrap();
        assert_eq!((w.tokens, w.source), (200_000, ContextSource::UserConfig));
    }

    #[test]
    fn falls_back_to_tables() {
        let cat = ConfiguredModelCatalog::new(
            ContextWindowConfig::default(),
            BuiltinModelCatalog::tables_only(),
        );
        let ev = ContextEvidence::with_model("claude-opus-5");
        let w = resolve_context_window(Provider::ClaudeCode, &ev, &cat).unwrap();
        assert_eq!((w.tokens, w.source), (1_000_000, ContextSource::ModelTable));
    }
}
