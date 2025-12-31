//! Execute command semantic intent classification for Codex
//!
//! Codex often uses shell commands to perform read operations (cat, sed -n, etc.)
//! This module provides semantic classification to distinguish read vs write vs other intents.

use agtrace_types::ToolKind;

/// Classify shell command to determine semantic intent
///
/// Returns Some(ToolKind) if the command has a specific semantic intent,
/// otherwise returns None to use the default Execute classification.
pub(crate) fn classify_execute_command(command: &str) -> Option<ToolKind> {
    let cmd = command.trim();
    let first_word = cmd.split_whitespace().next().unwrap_or("");

    // Check for search tools first (grep, rg, ag, ack)
    if is_search_command(cmd, first_word) {
        return Some(ToolKind::Search);
    }

    // Check for read commands
    if is_read_command(cmd, first_word) {
        return Some(ToolKind::Read);
    }

    // For write/build/test/etc., still return Execute
    None
}

fn is_search_command(cmd: &str, first_word: &str) -> bool {
    match first_word {
        // Search tools - but only if they're doing actual pattern searching
        "grep" | "ag" | "ack" => true,
        // rg (ripgrep) - only if NOT using file-listing options
        "rg" => {
            // rg --files, rg -l, rg --files-with-matches → file listing (Read, not Search)
            // rg pattern → pattern search (Search)
            !has_option(cmd, "--files")
                && !has_option(cmd, "-l")
                && !has_option(cmd, "--files-with-matches")
        }
        // Bash wrapper (check inner command)
        "bash" => {
            if let Some(inner) = extract_bash_inner_command(cmd) {
                let inner_first = inner.split_whitespace().next().unwrap_or("");
                is_search_command(&inner, inner_first)
            } else {
                false
            }
        }
        _ => false,
    }
}

fn is_read_command(cmd: &str, first_word: &str) -> bool {
    match first_word {
        // File content readers
        "cat" | "head" | "tail" | "less" | "more" => true,
        // File listing
        "ls" | "find" | "tree" | "fd" => true,
        // rg/grep in file-listing mode (not pattern search)
        "rg" if has_option(cmd, "--files")
            || has_option(cmd, "-l")
            || has_option(cmd, "--files-with-matches") =>
        {
            true
        }
        // File analysis
        "wc" | "diff" | "stat" | "file" => true,
        // Text processors (read-only mode)
        "sed" => {
            // sed is read-only unless it has -i or --in-place as option
            !has_option(cmd, "-i") && !has_option(cmd, "--in-place")
        }
        "awk" => {
            // awk is read-only unless it has output redirection
            !cmd.contains(">")
        }
        // Bash wrapper (check inner command)
        "bash" => {
            if let Some(inner) = extract_bash_inner_command(cmd) {
                let inner_first = inner.split_whitespace().next().unwrap_or("");
                is_read_command(&inner, inner_first)
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Extract inner command from bash wrapper (e.g., "bash -lc <command>")
fn extract_bash_inner_command(cmd: &str) -> Option<String> {
    if let Some(idx) = cmd.find("-lc") {
        let rest = cmd[idx + 3..].trim();
        return Some(rest.to_string());
    }
    if let Some(idx) = cmd.find("-c") {
        let rest = cmd[idx + 2..].trim();
        return Some(rest.to_string());
    }
    None
}

/// Check if a command has a specific option as a separate word (not part of a file path)
fn has_option(cmd: &str, option: &str) -> bool {
    cmd.split_whitespace().any(|word| {
        // Exact match or starts with the option (e.g., "-i" or "-i.bak")
        word == option
            || word.starts_with(option)
                && (word.len() == option.len()
                    || !word
                        .chars()
                        .nth(option.len())
                        .unwrap_or(' ')
                        .is_alphanumeric())
    })
}

/// Extract search pattern from search commands (grep, rg, ag, ack)
///
/// This is best-effort extraction for simple cases.
/// Returns None if the pattern cannot be reliably extracted.
pub(crate) fn extract_search_pattern(command: &str) -> Option<String> {
    let cmd = command.trim();
    let first_word = cmd.split_whitespace().next()?;

    match first_word {
        "grep" | "rg" | "ag" | "ack" => {
            // Find pattern after command and flags
            let rest = cmd.strip_prefix(first_word)?.trim();

            // Skip flags (words starting with -)
            let mut chars = rest.chars().peekable();
            let mut in_flags = true;

            while in_flags && chars.peek().is_some() {
                // Skip whitespace
                while chars.peek() == Some(&' ') {
                    chars.next();
                }

                // Check if this is a flag
                if chars.peek() == Some(&'-') {
                    // Skip this flag
                    while chars.peek().is_some() && chars.peek() != Some(&' ') {
                        chars.next();
                    }
                } else {
                    // Not a flag, this is the pattern
                    in_flags = false;
                }
            }

            let remainder: String = chars.collect();
            let remainder = remainder.trim();

            // Extract quoted or unquoted pattern
            if remainder.starts_with('"') {
                // Find closing quote
                let end = remainder[1..].find('"')?;
                Some(remainder[1..=end].to_string())
            } else if remainder.starts_with('\'') {
                // Find closing quote
                let end = remainder[1..].find('\'')?;
                Some(remainder[1..=end].to_string())
            } else {
                // Unquoted pattern - take first word
                remainder
                    .split_whitespace()
                    .next()
                    .map(|s| s.to_string())
            }
        }
        "bash" => {
            // For bash wrappers, extract inner command and recurse
            extract_bash_inner_command(cmd).and_then(|inner| extract_search_pattern(&inner))
        }
        _ => None,
    }
}

/// Extract file path from common read commands
///
/// This is best-effort extraction for simple cases.
/// Returns None if the file path cannot be reliably extracted.
pub(crate) fn extract_file_path(command: &str) -> Option<String> {
    let cmd = command.trim();
    let first_word = cmd.split_whitespace().next()?;

    match first_word {
        // Simple readers: last argument is usually the file
        "cat" | "head" | "tail" | "less" | "more" | "wc" | "file" => {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            // Skip the command and flags, take the last non-flag argument
            parts
                .iter()
                .rev()
                .find(|&&part| !part.starts_with('-'))
                .map(|s| s.to_string())
        }
        // sed: extract from patterns like "sed -n '1,100p' file.txt"
        "sed" => {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            // Find last non-flag, non-script argument
            parts
                .iter()
                .rev()
                .find(|&&part| !part.starts_with('-') && !part.contains(',') && part != "sed")
                .map(|s| s.to_string())
        }
        // grep: "grep pattern file"
        "grep" | "rg" | "ag" | "ack" => {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            // File is typically after the pattern
            if parts.len() >= 3 {
                parts.get(2).map(|s| s.to_string())
            } else {
                None
            }
        }
        "bash" => {
            // For bash wrappers, extract inner command and recurse
            extract_bash_inner_command(cmd).and_then(|inner| extract_file_path(&inner))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_search_commands() {
        assert_eq!(
            classify_execute_command("grep pattern file.txt"),
            Some(ToolKind::Search)
        );
        assert_eq!(
            classify_execute_command("rg -n \"context window\" docs -S"),
            Some(ToolKind::Search)
        );
        assert_eq!(
            classify_execute_command("ag \"pattern\" src/"),
            Some(ToolKind::Search)
        );
        assert_eq!(
            classify_execute_command("ack TODO crates/"),
            Some(ToolKind::Search)
        );
    }

    #[test]
    fn test_classify_read_commands() {
        assert_eq!(
            classify_execute_command("cat file.txt"),
            Some(ToolKind::Read)
        );
        assert_eq!(
            classify_execute_command("sed -n '1,200p' file.txt"),
            Some(ToolKind::Read)
        );
        assert_eq!(classify_execute_command("ls -la"), Some(ToolKind::Read));
        // rg --files is file listing, not search
        assert_eq!(
            classify_execute_command("rg --files docs"),
            Some(ToolKind::Read)
        );
        assert_eq!(
            classify_execute_command("rg -l pattern src/"),
            Some(ToolKind::Read)
        );
    }

    #[test]
    fn test_classify_write_commands() {
        // Write commands should return None (keep as Execute)
        assert_eq!(classify_execute_command("sed -i 's/foo/bar/' file.txt"), None);
        assert_eq!(classify_execute_command("mkdir -p dir"), None);
        assert_eq!(classify_execute_command("rm file.txt"), None);
    }

    #[test]
    fn test_classify_bash_wrapped() {
        assert_eq!(
            classify_execute_command("bash -lc cat file.txt"),
            Some(ToolKind::Read)
        );
        assert_eq!(
            classify_execute_command("bash -lc sed -n '1,100p' file.txt"),
            Some(ToolKind::Read)
        );
    }

    #[test]
    fn test_sed_with_path_containing_dash_i() {
        // Regression test: "extension-inspector" contains "-i"
        assert_eq!(
            classify_execute_command("sed -n 1,200p packages/extension-inspector/src/App.tsx"),
            Some(ToolKind::Read)
        );
    }

    #[test]
    fn test_extract_file_path() {
        assert_eq!(
            extract_file_path("cat file.txt"),
            Some("file.txt".to_string())
        );
        assert_eq!(
            extract_file_path("sed -n '1,100p' file.txt"),
            Some("file.txt".to_string())
        );
        assert_eq!(
            extract_file_path("grep pattern file.txt"),
            Some("file.txt".to_string())
        );
        assert_eq!(extract_file_path("ls"), None);
    }

    #[test]
    fn test_extract_file_path_bash_wrapped() {
        assert_eq!(
            extract_file_path("bash -lc cat file.txt"),
            Some("file.txt".to_string())
        );
    }

    #[test]
    fn test_extract_search_pattern() {
        assert_eq!(
            extract_search_pattern("rg -n \"context window\" docs -S"),
            Some("context window".to_string())
        );
        assert_eq!(
            extract_search_pattern("grep pattern file.txt"),
            Some("pattern".to_string())
        );
        assert_eq!(
            extract_search_pattern("ag TODO src/"),
            Some("TODO".to_string())
        );
        // rg --files has no pattern
        assert_eq!(extract_search_pattern("rg --files docs"), Some("docs".to_string()));
    }
}
