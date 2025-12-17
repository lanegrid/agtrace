use std::collections::HashMap;

/// Model specification with named fields for type safety
///
/// NOTE: Why struct instead of tuple (&str, u64)?
/// Tuples are position-dependent and lack semantic meaning:
/// - ("claude-3-5", 200_000) vs (200_000, "claude-3-5") - compiler can't catch order mistakes
/// - No field names make code less self-documenting
/// - Hard to extend (adding output_limit would create complex tuples)
///
/// Structs provide:
/// - Named fields prevent position errors (can't swap prefix and context_window)
/// - Self-documenting code (field names explain purpose)
/// - Easy to extend with new fields (e.g., output_limit, cache_support)
/// - IDE auto-completion works
/// - Compiler enforces all fields are provided
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelSpec {
    pub prefix: &'static str,
    pub context_window: u64,
}

impl ModelSpec {
    /// Create a new model specification
    ///
    /// NOTE: Why const fn?
    /// Allows construction at compile time in const context (MODEL_SPECS array).
    /// Zero runtime overhead - all values computed at compile time.
    pub const fn new(prefix: &'static str, context_window: u64) -> Self {
        Self {
            prefix,
            context_window,
        }
    }
}

/// Claude provider model specifications
///
/// NOTE: Why array of structs instead of HashMap::insert or tuples?
/// - Type safety: Named fields prevent position errors
/// - Immutability: Data defined at compile time, cannot be accidentally modified
/// - Duplicate detection: Tests verify no duplicate prefixes
/// - Maintainability: Clear structure makes it obvious what each value represents
/// - Extensibility: Easy to add new fields without breaking existing code
const MODEL_SPECS: &[ModelSpec] = &[
    // Claude 4.5 series (as of 2025-12-17)
    ModelSpec::new("claude-sonnet-4-5", 200_000),
    ModelSpec::new("claude-haiku-4-5", 200_000),
    ModelSpec::new("claude-opus-4-5", 200_000),
    // Claude 4 series
    ModelSpec::new("claude-sonnet-4", 200_000),
    ModelSpec::new("claude-haiku-4", 200_000),
    ModelSpec::new("claude-opus-4", 200_000),
    // Claude 3.5 series
    ModelSpec::new("claude-3-5", 200_000),
    // Claude 3 series (fallback)
    ModelSpec::new("claude-3", 200_000),
];

/// Returns model prefix -> context window limit mapping
pub fn get_model_limits() -> HashMap<&'static str, u64> {
    MODEL_SPECS
        .iter()
        .map(|spec| (spec.prefix, spec.context_window))
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

        // Verify key models are defined
        assert_eq!(limits.get("claude-sonnet-4-5"), Some(&200_000));
        assert_eq!(limits.get("claude-haiku-4-5"), Some(&200_000));
        assert_eq!(limits.get("claude-opus-4-5"), Some(&200_000));
        assert_eq!(limits.get("claude-sonnet-4"), Some(&200_000));
        assert_eq!(limits.get("claude-3-5"), Some(&200_000));
        assert_eq!(limits.get("claude-3"), Some(&200_000));
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
