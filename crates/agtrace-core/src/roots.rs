//! Provider home directories with environment overrides.
//!
//! - `AGTRACE_CLAUDE_HOME` overrides `~/.claude` (projects live in `<home>/projects`)
//! - `AGTRACE_CODEX_HOME` overrides `~/.codex` (rollouts live in `<home>/sessions`)
//!
//! The overrides let tests and the demo point agtrace at a synthetic fixture tree.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::path::expand_tilde;

/// Environment variable overriding the Claude Code home directory (`~/.claude`).
pub const CLAUDE_HOME_ENV: &str = "AGTRACE_CLAUDE_HOME";
/// Environment variable overriding the Codex home directory (`~/.codex`).
pub const CODEX_HOME_ENV: &str = "AGTRACE_CODEX_HOME";

fn resolve_home(
    env_value: Option<OsString>,
    user_home: Option<PathBuf>,
    default_dir: &str,
) -> Option<PathBuf> {
    if let Some(value) = env_value.filter(|v| !v.is_empty()) {
        return Some(expand_tilde(&value.to_string_lossy()));
    }
    user_home.map(|home| home.join(default_dir))
}

/// Claude Code home directory (`$AGTRACE_CLAUDE_HOME` or `~/.claude`).
pub fn claude_home() -> Option<PathBuf> {
    resolve_home(
        std::env::var_os(CLAUDE_HOME_ENV),
        dirs::home_dir(),
        ".claude",
    )
}

/// Claude Code transcript root (`<claude home>/projects`).
pub fn claude_projects_root() -> Option<PathBuf> {
    claude_home().map(|h| h.join("projects"))
}

/// Codex home directory (`$AGTRACE_CODEX_HOME` or `~/.codex`).
pub fn codex_home() -> Option<PathBuf> {
    resolve_home(std::env::var_os(CODEX_HOME_ENV), dirs::home_dir(), ".codex")
}

/// Codex rollout root (`<codex home>/sessions`).
pub fn codex_sessions_root() -> Option<PathBuf> {
    codex_home().map(|h| h.join("sessions"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_override_wins() {
        let got = resolve_home(
            Some(OsString::from("/fixtures/claude")),
            Some(PathBuf::from("/u")),
            ".claude",
        );
        assert_eq!(got, Some(PathBuf::from("/fixtures/claude")));
    }

    #[test]
    fn empty_env_falls_back_to_home() {
        let got = resolve_home(Some(OsString::new()), Some(PathBuf::from("/u")), ".codex");
        assert_eq!(got, Some(PathBuf::from("/u/.codex")));
    }

    #[test]
    fn default_is_home_dot_dir() {
        let got = resolve_home(None, Some(PathBuf::from("/u")), ".claude");
        assert_eq!(got, Some(PathBuf::from("/u/.claude")));
        assert_eq!(resolve_home(None, None, ".claude"), None);
    }
}
