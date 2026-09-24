//! Claude model knowledge: built-in context window table (consumed by the context
//! resolver through [`crate::BuiltinModelCatalog`], never by the decoder).
//!
//! Keys are model-id prefixes matched longest-first against the normalized id
//! (`[1m]` suffix and `-YYYYMMDD` snapshot stripped). Values are the default window on
//! the Anthropic API. Bedrock/Vertex deployments of native-1M models may run at 200k;
//! users override that in `config.toml` `[context_window]`.

/// Model prefix → context window (tokens).
pub const CONTEXT_WINDOWS: &[(&str, u64)] = &[
    // Claude 5.x: native 1M context.
    ("claude-opus-5", 1_000_000),
    ("claude-opus-5-5", 1_000_000),
    ("claude-fable-5", 1_000_000),
    ("claude-fable-5-1", 1_000_000),
    ("claude-mythos-5", 1_000_000),
    ("claude-sonnet-5", 1_000_000),
    // Claude 4.6+: native 1M context.
    ("claude-opus-4-6", 1_000_000),
    ("claude-opus-4-7", 1_000_000),
    ("claude-opus-4-8", 1_000_000),
    ("claude-sonnet-4-6", 1_000_000),
    // Claude 4.5 and older: 200k.
    ("claude-haiku-4-5", 200_000),
    ("claude-opus-4", 200_000),
    ("claude-sonnet-4", 200_000),
    ("claude-haiku-4", 200_000),
    ("claude-3", 200_000),
];

/// Built-in context window for a Claude model id.
pub fn context_window(model: &str) -> Option<u64> {
    agtrace_types::longest_prefix_lookup(CONTEXT_WINDOWS, model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_duplicate_prefixes() {
        let set: HashSet<_> = CONTEXT_WINDOWS.iter().map(|(p, _)| p).collect();
        assert_eq!(set.len(), CONTEXT_WINDOWS.len());
    }

    #[test]
    fn current_models_are_1m() {
        for m in [
            "claude-opus-5",
            "claude-opus-5-5",
            "claude-opus-5-5[1m]",
            "claude-fable-5",
            "claude-fable-5-1",
            "claude-sonnet-5",
            "claude-opus-4-8",
        ] {
            assert_eq!(context_window(m), Some(1_000_000), "{m}");
        }
    }

    #[test]
    fn older_models_are_200k() {
        for m in [
            "claude-haiku-4-5-20251001",
            "claude-sonnet-4-5-20250929",
            "claude-opus-4-5",
            "claude-opus-4-1",
            "claude-3-5-sonnet-20241022",
        ] {
            assert_eq!(context_window(m), Some(200_000), "{m}");
        }
    }

    #[test]
    fn unknown_model_is_none() {
        assert_eq!(context_window("<synthetic>"), None);
        assert_eq!(context_window("gpt-5"), None);
    }
}
