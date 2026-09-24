//! Lenient reader for Codex's model cache (`<codex home>/models_cache.json`).
//!
//! Shape (only the fields we use; everything else is ignored):
//! `{ "models": [ { "slug", "context_window", "max_context_window",
//!                  "effective_context_window_percent", ... } ] }`
//!
//! Self-contained: no dependency on the Codex decoder.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// File name inside the Codex home directory.
pub const MODELS_CACHE_FILE: &str = "models_cache.json";

/// Context window facts for one Codex model slug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodexModelInfo {
    /// Raw context window (e.g. 272_000).
    pub context_window: Option<u64>,
    /// Maximum configurable window (e.g. 872_000).
    pub max_context_window: Option<u64>,
    /// Percent of `context_window` usable before auto-compaction (e.g. 95).
    pub effective_context_window_percent: Option<u64>,
}

impl CodexModelInfo {
    /// Effective window: `floor(context_window * effective_percent / 100)`,
    /// or `context_window` when no percent is given.
    pub fn effective_context_window(&self) -> Option<u64> {
        let window = self.context_window.filter(|w| *w > 0)?;
        match self.effective_context_window_percent {
            Some(pct) if pct > 0 && pct <= 100 => Some(window * pct / 100),
            _ => Some(window),
        }
    }
}

/// Parsed `models_cache.json`, keyed by lowercase slug.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodexModelsCache {
    models: HashMap<String, CodexModelInfo>,
}

#[derive(Deserialize)]
struct RawCache {
    #[serde(default)]
    models: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct RawModel {
    slug: String,
    #[serde(default)]
    context_window: Option<u64>,
    #[serde(default)]
    max_context_window: Option<u64>,
    #[serde(default)]
    effective_context_window_percent: Option<u64>,
}

impl CodexModelsCache {
    /// Default location: `<codex home>/models_cache.json` (honors `AGTRACE_CODEX_HOME`).
    pub fn default_path() -> Option<PathBuf> {
        agtrace_core::codex_home().map(|h| h.join(MODELS_CACHE_FILE))
    }

    /// Load from the default location. Missing or unreadable file ⇒ `None`.
    pub fn load_default() -> Option<Self> {
        Self::load(&Self::default_path()?)
    }

    /// Load from a file. Missing / unreadable / non-JSON file ⇒ `None`.
    pub fn load(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        Self::parse(&text)
    }

    /// Parse leniently: entries that fail to decode (no slug, wrong types) are skipped.
    pub fn parse(text: &str) -> Option<Self> {
        let raw: RawCache = serde_json::from_str(text).ok()?;
        let models = raw
            .models
            .into_iter()
            .filter_map(|v| serde_json::from_value::<RawModel>(v).ok())
            .map(|m| {
                (
                    m.slug.trim().to_ascii_lowercase(),
                    CodexModelInfo {
                        context_window: m.context_window,
                        max_context_window: m.max_context_window,
                        effective_context_window_percent: m.effective_context_window_percent,
                    },
                )
            })
            .collect();
        Some(Self { models })
    }

    /// Info for an exact slug (case-insensitive).
    pub fn get(&self, slug: &str) -> Option<&CodexModelInfo> {
        self.models.get(&slug.trim().to_ascii_lowercase())
    }

    /// Effective context window for a slug.
    pub fn effective_context_window(&self, slug: &str) -> Option<u64> {
        self.get(slug)?.effective_context_window()
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "client_version": "0.153.0",
      "etag": "W/\"synthetic\"",
      "fetched_at": "2026-09-20T00:00:00Z",
      "models": [
        {"slug": "gpt-6-astra", "display_name": "GPT-6 Astra", "context_window": 272000,
         "max_context_window": 872000, "effective_context_window_percent": 95,
         "auto_compact_token_limit": null, "truncation_policy": {"mode": "tokens", "limit": 10000}},
        {"slug": "gpt-5.5", "context_window": 272000, "max_context_window": 272000},
        {"slug": "broken", "context_window": "lots"},
        {"display_name": "no slug"}
      ]
    }"#;

    #[test]
    fn parses_effective_window() {
        let cache = CodexModelsCache::parse(SAMPLE).unwrap();
        assert_eq!(cache.effective_context_window("gpt-6-astra"), Some(258_400));
        assert_eq!(cache.effective_context_window("GPT-6-ASTRA"), Some(258_400));
        let info = cache.get("gpt-6-astra").unwrap();
        assert_eq!(info.max_context_window, Some(872_000));
    }

    #[test]
    fn missing_percent_uses_raw_window() {
        let cache = CodexModelsCache::parse(SAMPLE).unwrap();
        assert_eq!(cache.effective_context_window("gpt-5.5"), Some(272_000));
    }

    #[test]
    fn bad_entries_are_skipped() {
        let cache = CodexModelsCache::parse(SAMPLE).unwrap();
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.effective_context_window("broken"), None);
        assert_eq!(cache.effective_context_window("unknown"), None);
    }

    #[test]
    fn invalid_json_is_none() {
        assert!(CodexModelsCache::parse("not json").is_none());
        assert!(CodexModelsCache::parse("{}").unwrap().is_empty());
    }

    #[test]
    fn missing_file_is_none() {
        assert!(CodexModelsCache::load(Path::new("/nonexistent/models_cache.json")).is_none());
    }
}
