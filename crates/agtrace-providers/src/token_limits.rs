// NOTE: Architecture decision - Provider-specific model definitions
// Model specifications are distributed across provider modules (claude/models.rs,
// codex/models.rs, gemini/models.rs) rather than centralized here because:
// 1. Maintainability: Each provider can be updated independently without touching other providers
// 2. Extensibility: Adding a new provider only requires creating a new module and adding one line here
// 3. Separation of concerns: Provider-specific knowledge stays with the provider
// This follows the "distributed definition, centralized resolution" pattern.

use crate::claude::models as claude_models;
use crate::codex::models as codex_models;
use crate::gemini::models as gemini_models;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelSpec {
    pub max_tokens: u64,
    /// Compaction buffer percentage (0-100)
    /// When input tokens exceed (100% - compaction_buffer_pct), compaction is triggered
    /// Example: Claude Code has 22.5% buffer, so compaction happens at 77.5% input usage
    pub compaction_buffer_pct: f64,
}

/// Resolve model context window limit using longest prefix matching
///
/// NOTE: Why longest prefix matching instead of exact matching?
/// Model providers release new minor versions frequently (e.g., claude-sonnet-4-5-20250929).
/// Exact matching would require updating our codebase for every minor release, which is:
/// - High maintenance burden for OSS contributors
/// - Fragile (breaks on unknown versions)
/// - Unnecessary (minor versions rarely change context limits)
///
/// Longest prefix matching allows us to:
/// - Define "claude-sonnet-4-5" once and match all dated variants (20250929, 20260101, etc.)
/// - Gracefully handle unknown models (return None instead of incorrect data)
/// - Reduce false positives by preferring more specific matches
///
/// Resolution strategy:
/// 1. Collect all provider-defined model prefixes
/// 2. Find the longest prefix match for the given model name
/// 3. Return the corresponding limit, or None if no match found
///
/// Example:
/// - "claude-sonnet-4-5-20250929" matches "claude-sonnet-4-5" (200K)
/// - "gpt-5.1-codex-max-2025" matches "gpt-5.1-codex-max" (400K)
/// - "gemini-2.5-flash-exp" matches "gemini-2.5-flash" (1M)
pub fn resolve_model_limit(model_name: &str) -> Option<ModelSpec> {
    // NOTE: Why aggregate on every call instead of using lazy_static?
    // The aggregation overhead is negligible (< 100 entries, ~microseconds) compared to
    // the benefits of simplicity and testability. If profiling shows this is a bottleneck,
    // we can optimize with lazy_static/OnceCell later. YAGNI principle applies here.
    let all_limits: HashMap<&str, (u64, f64)> = [
        claude_models::get_model_limits(),
        codex_models::get_model_limits(),
        gemini_models::get_model_limits(),
    ]
    .into_iter()
    .flat_map(|map| map.into_iter())
    .collect();

    // Longest prefix matching algorithm
    // NOTE: This is O(n) where n = number of defined model prefixes (~30-50).
    // We prefer readability over premature optimization (e.g., trie structures).
    let mut best_match: Option<(u64, f64)> = None;
    let mut best_len = 0;

    for (prefix, &(limit, buffer)) in &all_limits {
        if model_name.starts_with(prefix) && prefix.len() > best_len {
            best_match = Some((limit, buffer));
            best_len = prefix.len();
        }
    }

    best_match.map(|(max_tokens, compaction_buffer_pct)| ModelSpec {
        max_tokens,
        compaction_buffer_pct,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_models() {
        // Exact match
        assert_eq!(
            resolve_model_limit("claude-sonnet-4-5"),
            Some(ModelSpec {
                max_tokens: 200_000,
                compaction_buffer_pct: 22.5
            })
        );

        // Prefix match (minor version)
        assert_eq!(
            resolve_model_limit("claude-sonnet-4-5-20250929"),
            Some(ModelSpec {
                max_tokens: 200_000,
                compaction_buffer_pct: 22.5
            })
        );
        assert_eq!(
            resolve_model_limit("claude-haiku-4-5-20251001"),
            Some(ModelSpec {
                max_tokens: 200_000,
                compaction_buffer_pct: 22.5
            })
        );

        // Claude 3.5 series
        assert_eq!(
            resolve_model_limit("claude-3-5-sonnet-20241022"),
            Some(ModelSpec {
                max_tokens: 200_000,
                compaction_buffer_pct: 22.5
            })
        );

        // Claude 3 fallback
        assert_eq!(
            resolve_model_limit("claude-3-opus-20240229"),
            Some(ModelSpec {
                max_tokens: 200_000,
                compaction_buffer_pct: 22.5
            })
        );
    }

    #[test]
    fn test_codex_models() {
        // GPT-5.2
        assert_eq!(
            resolve_model_limit("gpt-5.2"),
            Some(ModelSpec {
                max_tokens: 400_000,
                compaction_buffer_pct: 0.0
            })
        );

        // GPT-5.1 series
        assert_eq!(
            resolve_model_limit("gpt-5.1-codex-max"),
            Some(ModelSpec {
                max_tokens: 400_000,
                compaction_buffer_pct: 0.0
            })
        );
        assert_eq!(
            resolve_model_limit("gpt-5.1-codex"),
            Some(ModelSpec {
                max_tokens: 400_000,
                compaction_buffer_pct: 0.0
            })
        );

        // GPT-5 series
        assert_eq!(
            resolve_model_limit("gpt-5-codex"),
            Some(ModelSpec {
                max_tokens: 400_000,
                compaction_buffer_pct: 0.0
            })
        );
        assert_eq!(
            resolve_model_limit("gpt-5"),
            Some(ModelSpec {
                max_tokens: 400_000,
                compaction_buffer_pct: 0.0
            })
        );
    }

    #[test]
    fn test_gemini_models() {
        // Gemini 2.5 series
        assert_eq!(
            resolve_model_limit("gemini-2.5-pro"),
            Some(ModelSpec {
                max_tokens: 1_048_576,
                compaction_buffer_pct: 0.0
            })
        );
        assert_eq!(
            resolve_model_limit("gemini-2.5-flash"),
            Some(ModelSpec {
                max_tokens: 1_048_576,
                compaction_buffer_pct: 0.0
            })
        );

        // Gemini 2.0 series
        assert_eq!(
            resolve_model_limit("gemini-2.0-flash"),
            Some(ModelSpec {
                max_tokens: 1_048_576,
                compaction_buffer_pct: 0.0
            })
        );
    }

    #[test]
    fn test_unknown_model() {
        assert_eq!(resolve_model_limit("unknown-model"), None);
        assert_eq!(resolve_model_limit("gpt-3"), None);
        assert_eq!(resolve_model_limit("claude-2"), None);
    }

    #[test]
    fn test_longest_prefix_matching() {
        // Should match "gpt-5.1-codex-max" (400K) not "gpt-5.1" (400K)
        // In this case both have the same value, but the algorithm should prefer the longest match
        let spec = resolve_model_limit("gpt-5.1-codex-max-2025");
        assert_eq!(
            spec,
            Some(ModelSpec {
                max_tokens: 400_000,
                compaction_buffer_pct: 0.0
            })
        );

        // Should match "claude-sonnet-4-5" not "claude-sonnet-4"
        let spec = resolve_model_limit("claude-sonnet-4-5-20250929");
        assert_eq!(
            spec,
            Some(ModelSpec {
                max_tokens: 200_000,
                compaction_buffer_pct: 22.5
            })
        );
    }

    #[test]
    fn test_prefix_match_with_suffix() {
        // Prefix match should work with any suffix
        assert_eq!(
            resolve_model_limit("claude-3-5-sonnet-custom-version"),
            Some(ModelSpec {
                max_tokens: 200_000,
                compaction_buffer_pct: 22.5
            })
        );
        assert_eq!(
            resolve_model_limit("gpt-5.1-codex-experimental"),
            Some(ModelSpec {
                max_tokens: 400_000,
                compaction_buffer_pct: 0.0
            })
        );
    }
}
