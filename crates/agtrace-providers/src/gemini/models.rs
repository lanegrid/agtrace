use std::collections::HashMap;

/// Model specification with named fields for type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelSpec {
    pub prefix: &'static str,
    pub context_window: u64,
}

impl ModelSpec {
    pub const fn new(prefix: &'static str, context_window: u64) -> Self {
        Self {
            prefix,
            context_window,
        }
    }
}

/// Gemini provider model specifications
const MODEL_SPECS: &[ModelSpec] = &[
    // Gemini 2.5 series (as of 2025-12-17)
    ModelSpec::new("gemini-2.5-pro", 1_048_576),
    ModelSpec::new("gemini-2.5-flash", 1_048_576),
    ModelSpec::new("gemini-2.5-flash-lite", 1_048_576),
    // Gemini 2.0 series
    ModelSpec::new("gemini-2.0-flash", 1_048_576),
    ModelSpec::new("gemini-2.0-flash-lite", 1_048_576),
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

        // Verify Gemini 2.5 series
        assert_eq!(limits.get("gemini-2.5-pro"), Some(&1_048_576));
        assert_eq!(limits.get("gemini-2.5-flash"), Some(&1_048_576));
        assert_eq!(limits.get("gemini-2.5-flash-lite"), Some(&1_048_576));

        // Verify Gemini 2.0 series
        assert_eq!(limits.get("gemini-2.0-flash"), Some(&1_048_576));
        assert_eq!(limits.get("gemini-2.0-flash-lite"), Some(&1_048_576));
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
