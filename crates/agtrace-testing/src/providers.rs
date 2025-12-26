//! Provider definitions for testing logic.
//!
//! This module provides type-safe provider handling for tests,
//! abstracting away provider-specific directory structures and
//! configuration details.

/// Supported test providers.
///
/// Each variant represents a different AI agent provider that agtrace supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestProvider {
    /// Claude Code provider
    Claude,
    /// Gemini provider
    Gemini,
    /// Codex provider
    Codex,
}

impl TestProvider {
    /// Get the provider name as used in config.toml and CLI.
    ///
    /// # Example
    /// ```
    /// # use agtrace_testing::providers::TestProvider;
    /// assert_eq!(TestProvider::Claude.name(), "claude_code");
    /// assert_eq!(TestProvider::Gemini.name(), "gemini");
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            TestProvider::Claude => "claude_code",
            TestProvider::Gemini => "gemini",
            TestProvider::Codex => "codex",
        }
    }

    /// Get the default log directory name for this provider.
    ///
    /// This is the directory name relative to the temp root where
    /// the provider's logs are stored (e.g., `.claude`, `.gemini`).
    pub fn default_log_dir_name(&self) -> &'static str {
        match self {
            TestProvider::Claude => ".claude",
            TestProvider::Gemini => ".gemini",
            TestProvider::Codex => ".codex",
        }
    }

    /// Get the sample filename for this provider.
    ///
    /// Returns the filename in the samples directory that contains
    /// example data for this provider.
    pub fn sample_filename(&self) -> &'static str {
        match self {
            TestProvider::Claude => "claude_session.jsonl",
            TestProvider::Gemini => "gemini_session.json",
            TestProvider::Codex => "codex_session.jsonl",
        }
    }

    /// Get all supported providers.
    pub fn all() -> &'static [TestProvider] {
        &[
            TestProvider::Claude,
            TestProvider::Gemini,
            TestProvider::Codex,
        ]
    }
}

impl std::fmt::Display for TestProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
