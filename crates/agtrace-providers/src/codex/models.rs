use std::collections::HashMap;

/// Model specification with named fields for type safety
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelSpec {
    pub prefix: &'static str,
    pub context_window: u64,
    /// Compaction buffer percentage (0-100)
    /// When input tokens exceed (100% - compaction_buffer_pct), compaction is triggered
    pub compaction_buffer_pct: f64,
}

impl ModelSpec {
    pub const fn new(
        prefix: &'static str,
        context_window: u64,
        compaction_buffer_pct: f64,
    ) -> Self {
        Self {
            prefix,
            context_window,
            compaction_buffer_pct,
        }
    }
}

/// Compaction buffer percentage for Codex/OpenAI models
/// NOTE: Set to 0 as the actual compaction behavior is not yet known
const COMPACTION_BUFFER_PCT: f64 = 0.0;

/// Codex/OpenAI provider model specifications
const MODEL_SPECS: &[ModelSpec] = &[
    // GPT-5.2 series (as of 2025-12-17)
    ModelSpec::new("gpt-5.2", 400_000, COMPACTION_BUFFER_PCT),
    // GPT-5.1 series
    ModelSpec::new("gpt-5.1-codex-max", 400_000, COMPACTION_BUFFER_PCT),
    ModelSpec::new("gpt-5.1-codex-mini", 400_000, COMPACTION_BUFFER_PCT),
    ModelSpec::new("gpt-5.1-codex", 400_000, COMPACTION_BUFFER_PCT),
    ModelSpec::new("gpt-5.1", 400_000, COMPACTION_BUFFER_PCT),
    // GPT-5 series
    ModelSpec::new("gpt-5-codex-mini", 400_000, COMPACTION_BUFFER_PCT),
    ModelSpec::new("gpt-5-codex", 400_000, COMPACTION_BUFFER_PCT),
    ModelSpec::new("gpt-5", 400_000, COMPACTION_BUFFER_PCT),
];

/// Returns model prefix -> (context window, compaction buffer %) mapping
pub fn get_model_limits() -> HashMap<&'static str, (u64, f64)> {
    MODEL_SPECS
        .iter()
        .map(|spec| {
            (
                spec.prefix,
                (spec.context_window, spec.compaction_buffer_pct),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_no_duplicate_prefixes() {
        let prefixes: Vec<&str> = MODEL_SPECS.iter().map(|spec| spec.prefix).collect();
        let unique_prefixes: HashSet<&str> = prefixes.iter().copied().collect();

        assert_eq!(
            prefixes.len(),
            unique_prefixes.len(),
            "Duplicate prefixes found in MODEL_SPECS: {:?}",
            prefixes
                .iter()
                .enumerate()
                .filter(|(i, p)| prefixes.iter().skip(i + 1).any(|other| other == *p))
                .map(|(_, p)| p)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_model_limits_coverage() {
        let limits = get_model_limits();

        // Verify GPT-5.2 series
        assert_eq!(limits.get("gpt-5.2"), Some(&(400_000, 0.0)));

        // Verify GPT-5.1 series
        assert_eq!(limits.get("gpt-5.1-codex-max"), Some(&(400_000, 0.0)));
        assert_eq!(limits.get("gpt-5.1-codex-mini"), Some(&(400_000, 0.0)));
        assert_eq!(limits.get("gpt-5.1-codex"), Some(&(400_000, 0.0)));
        assert_eq!(limits.get("gpt-5.1"), Some(&(400_000, 0.0)));

        // Verify GPT-5 series
        assert_eq!(limits.get("gpt-5-codex-mini"), Some(&(400_000, 0.0)));
        assert_eq!(limits.get("gpt-5-codex"), Some(&(400_000, 0.0)));
        assert_eq!(limits.get("gpt-5"), Some(&(400_000, 0.0)));
    }

    #[test]
    fn test_all_specs_converted() {
        let limits = get_model_limits();
        assert_eq!(
            limits.len(),
            MODEL_SPECS.len(),
            "HashMap size should match MODEL_SPECS length"
        );
    }
}
