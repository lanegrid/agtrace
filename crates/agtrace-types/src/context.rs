//! Context window domain types (design §3).
//!
//! The resolver itself lives in `agtrace-engine::context`; this module only holds the
//! shared vocabulary: where a window size came from ([`ContextSource`]), the resolved
//! value ([`ContextWindow`]) and the injected model knowledge ([`ModelCatalog`]).

use serde::{Deserialize, Serialize};

use crate::Provider;

/// Where a resolved context window size came from, in resolution priority order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSource {
    /// agtrace `config.toml` `[context_window]` override.
    UserConfig,
    /// Explicit number reported in the log (Codex `model_context_window`).
    Log,
    /// Extended-context marker: `[1m]` model suffix or "(1M context)" marketing name.
    ModelMarker,
    /// Provider-local model cache (`~/.codex/models_cache.json`).
    ProviderCache,
    /// Built-in per-model table.
    ModelTable,
    /// Derived from observed usage (peak context / compaction pre-tokens) bumped to a tier.
    Observed,
}

impl ContextSource {
    /// Short provenance label for UIs: `cfg`, `log`, `1m`, `cache`, `table`, `obs`.
    pub fn label(&self) -> &'static str {
        match self {
            ContextSource::UserConfig => "cfg",
            ContextSource::Log => "log",
            ContextSource::ModelMarker => "1m",
            ContextSource::ProviderCache => "cache",
            ContextSource::ModelTable => "table",
            ContextSource::Observed => "obs",
        }
    }
}

/// A resolved context window with provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextWindow {
    /// Window size in tokens.
    pub tokens: u64,
    /// Which layer produced `tokens`.
    pub source: ContextSource,
    /// Model the window was resolved for, if known.
    pub model: Option<String>,
    /// When `source == Observed` because the observed usage exceeded a lower-priority
    /// resolution, the source that was overruled. `None` means nothing else resolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overruled: Option<ContextSource>,
}

impl ContextWindow {
    /// Provenance label; `obs?` when the window is a pure guess from observed usage.
    pub fn provenance(&self) -> &'static str {
        match (self.source, self.overruled) {
            (ContextSource::Observed, None) => "obs?",
            (source, _) => source.label(),
        }
    }

    /// Fraction of the window occupied by `context_tokens` (not clamped).
    pub fn usage_ratio(&self, context_tokens: u64) -> f64 {
        if self.tokens == 0 {
            return 0.0;
        }
        context_tokens as f64 / self.tokens as f64
    }
}

/// Model knowledge injected into the context resolver.
///
/// Implemented in `agtrace-providers` (static tables + provider caches) and wrapped by
/// `agtrace-runtime` with the user's config overrides.
pub trait ModelCatalog: Send + Sync {
    /// User-configured window for this provider / model (skips the observed floor).
    fn user_override(&self, provider: Provider, model: Option<&str>) -> Option<u64>;
    /// Window from a provider-local cache (effective size).
    fn provider_cache(&self, provider: Provider, model: &str) -> Option<u64>;
    /// Window from the built-in model table.
    fn table(&self, provider: Provider, model: &str) -> Option<u64>;
}

/// Normalize a model id for lookups: trims, lowercases, strips a trailing bracketed
/// variant suffix (`claude-opus-5-5[1m]` → `claude-opus-5-5`) and a trailing
/// `-YYYYMMDD` date snapshot (`claude-haiku-4-5-20251001` → `claude-haiku-4-5`).
pub fn normalize_model_id(model: &str) -> String {
    let mut s = model.trim().to_ascii_lowercase();
    if s.ends_with(']')
        && let Some(open) = s.rfind('[')
    {
        s.truncate(open);
    }
    let bytes = s.as_bytes();
    if bytes.len() > 9 {
        let tail = &bytes[bytes.len() - 9..];
        if tail[0] == b'-' && tail[1..].iter().all(u8::is_ascii_digit) {
            s.truncate(s.len() - 9);
        }
    }
    s.trim().to_string()
}

/// Whether a model id carries the `[1m]` extended-context suffix.
pub fn has_extended_context_suffix(model: &str) -> bool {
    model.trim().to_ascii_lowercase().ends_with("[1m]")
}

/// Longest-prefix lookup of a normalized model id in a `(prefix, tokens)` table.
pub fn longest_prefix_lookup(table: &[(&str, u64)], model: &str) -> Option<u64> {
    let key = normalize_model_id(model);
    table
        .iter()
        .filter(|(prefix, _)| key.starts_with(prefix))
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(_, tokens)| *tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_suffix_and_date() {
        assert_eq!(normalize_model_id("claude-opus-5-5[1m]"), "claude-opus-5-5");
        assert_eq!(
            normalize_model_id("claude-haiku-4-5-20251001"),
            "claude-haiku-4-5"
        );
        assert_eq!(normalize_model_id(" GPT-5.6-Sol "), "gpt-5.6-sol");
        assert_eq!(normalize_model_id("claude-opus-5"), "claude-opus-5");
    }

    #[test]
    fn extended_suffix() {
        assert!(has_extended_context_suffix("claude-opus-5-5[1m]"));
        assert!(has_extended_context_suffix("opus[1M]"));
        assert!(!has_extended_context_suffix("claude-opus-5-5"));
    }

    #[test]
    fn longest_prefix_wins() {
        let table = [("gpt-5", 1), ("gpt-5.6", 2)];
        assert_eq!(longest_prefix_lookup(&table, "gpt-5.6-sol"), Some(2));
        assert_eq!(longest_prefix_lookup(&table, "gpt-5.1"), Some(1));
        assert_eq!(longest_prefix_lookup(&table, "o3"), None);
    }

    #[test]
    fn provenance_labels() {
        let w = ContextWindow {
            tokens: 1,
            source: ContextSource::Observed,
            model: None,
            overruled: None,
        };
        assert_eq!(w.provenance(), "obs?");
        let w = ContextWindow {
            overruled: Some(ContextSource::ModelTable),
            ..w
        };
        assert_eq!(w.provenance(), "obs");
    }
}
