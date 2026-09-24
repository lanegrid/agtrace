//! Codex model knowledge: built-in context window table (consumed by the context
//! resolver through [`crate::BuiltinModelCatalog`]).
//!
//! Values are the *effective* window Codex reports in its logs
//! (`model_context_window` = `context_window` × `effective_context_window_percent`,
//! 272_000 × 95% = 258_400). The live source of truth is the log itself
//! (`ContextWindowHint::Explicit`) and `~/.codex/models_cache.json`; this table is the
//! last fallback.

/// Model prefix → effective context window (tokens).
pub const CONTEXT_WINDOWS: &[(&str, u64)] = &[
    ("gpt-6", 258_400),
    ("gpt-5.6", 258_400),
    ("gpt-5.5", 258_400),
    ("gpt-5", 258_400),
];

/// Built-in effective context window for a Codex model slug.
pub fn context_window(model: &str) -> Option<u64> {
    agtrace_types::longest_prefix_lookup(CONTEXT_WINDOWS, model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_slugs() {
        for m in [
            "gpt-6-astra",
            "gpt-5.6-sol",
            "gpt-5.6-luna",
            "gpt-5.5",
            "gpt-5.1-codex-max",
        ] {
            assert_eq!(context_window(m), Some(258_400), "{m}");
        }
    }

    #[test]
    fn unknown_slug() {
        assert_eq!(context_window("o3"), None);
    }
}
