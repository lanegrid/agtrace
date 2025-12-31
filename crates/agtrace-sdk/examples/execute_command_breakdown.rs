//! Execute command breakdown example: Analyze semantic intent of shell commands
//!
//! This example demonstrates:
//! - Classifying Execute commands by semantic intent (read, write, build, test, etc.)
//! - Identifying read-oriented commands (cat, sed -n, head, tail, grep, ls, etc.)
//! - Identifying write-oriented commands (sed -i, echo >, rm, mkdir, etc.)
//! - Showing statistics per provider
//!
//! Run with: cargo run --release -p agtrace-sdk --example execute_command_breakdown

use agtrace_sdk::{
    Client,
    types::{SessionFilter, ToolCallPayload},
};
use std::collections::HashMap;

/// Semantic intent of an Execute command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ExecuteIntent {
    /// Read-oriented: cat, sed -n, head, tail, grep, ls, find, wc, diff, etc.
    Read,
    /// Write-oriented: sed -i, echo >, rm, mkdir, mv, cp, etc.
    Write,
    /// Build/compile: cargo build, npm build, make, etc.
    Build,
    /// Test: cargo test, npm test, pytest, etc.
    Test,
    /// Version control: git, gh, etc.
    Git,
    /// Other commands
    Other,
}

impl ExecuteIntent {
    fn classify(command: &str) -> Self {
        let cmd = command.trim();
        let first_word = cmd.split_whitespace().next().unwrap_or("");

        // Check for read-oriented commands
        if Self::is_read_command(cmd, first_word) {
            return ExecuteIntent::Read;
        }

        // Check for write-oriented commands
        if Self::is_write_command(cmd, first_word) {
            return ExecuteIntent::Write;
        }

        // Check for build commands
        if cmd.contains("build") || cmd.contains("compile") || first_word == "make" {
            return ExecuteIntent::Build;
        }

        // Check for test commands
        if cmd.contains("test") || cmd.contains("pytest") || cmd.contains("jest") {
            return ExecuteIntent::Test;
        }

        // Check for git commands
        if first_word == "git" || first_word == "gh" {
            return ExecuteIntent::Git;
        }

        ExecuteIntent::Other
    }

    fn is_read_command(cmd: &str, first_word: &str) -> bool {
        match first_word {
            // File content readers
            "cat" | "head" | "tail" | "less" | "more" => true,
            // Search tools
            "grep" | "rg" | "ag" | "ack" => true,
            // File listing
            "ls" | "find" | "tree" | "fd" => true,
            // File analysis
            "wc" | "diff" | "stat" | "file" => true,
            // Text processors (read-only mode)
            "sed" => {
                // sed is read-only unless it has -i or --in-place as option
                // Check for -i as a separate word to avoid false positives in file paths
                !Self::has_option(cmd, "-i") && !Self::has_option(cmd, "--in-place")
            }
            "awk" => {
                // awk is read-only unless it has output redirection
                !cmd.contains(">")
            }
            // Bash wrapper (check inner command)
            "bash" => {
                // Extract inner command from "bash -lc <command>"
                if let Some(inner) = Self::extract_bash_inner_command(cmd) {
                    let inner_first = inner.split_whitespace().next().unwrap_or("");
                    Self::is_read_command(&inner, inner_first)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn is_write_command(cmd: &str, first_word: &str) -> bool {
        // First check if it's a read command (more specific check)
        // to avoid false positives with sed/awk
        if Self::is_read_command(cmd, first_word) {
            return false;
        }

        match first_word {
            // File operations
            "rm" | "mv" | "cp" | "mkdir" | "rmdir" | "touch" => true,
            // Write redirections
            "echo" | "printf" if cmd.contains(">") => true,
            // In-place editors (only if not already classified as read)
            "sed" if Self::has_option(cmd, "-i") || Self::has_option(cmd, "--in-place") => true,
            "awk" if cmd.contains(">") => true,
            // Bash wrapper (check inner command)
            "bash" => {
                if let Some(inner) = Self::extract_bash_inner_command(cmd) {
                    let inner_first = inner.split_whitespace().next().unwrap_or("");
                    Self::is_write_command(&inner, inner_first)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn extract_bash_inner_command(cmd: &str) -> Option<String> {
        // Match patterns like "bash -lc <command>"
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
            word == option || word.starts_with(&format!("{}", option))
                && (word.len() == option.len() || !word.chars().nth(option.len()).unwrap_or(' ').is_alphanumeric())
        })
    }
}

impl std::fmt::Display for ExecuteIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecuteIntent::Read => write!(f, "Read"),
            ExecuteIntent::Write => write!(f, "Write"),
            ExecuteIntent::Build => write!(f, "Build"),
            ExecuteIntent::Test => write!(f, "Test"),
            ExecuteIntent::Git => write!(f, "Git"),
            ExecuteIntent::Other => write!(f, "Other"),
        }
    }
}

/// Statistics for a single provider
#[derive(Default)]
struct ProviderStats {
    total_execute: usize,
    intent_counts: HashMap<ExecuteIntent, usize>,
    intent_examples: HashMap<ExecuteIntent, Vec<String>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== agtrace SDK: Execute Command Breakdown ===\n");

    // 1. Connect to workspace
    let client = Client::connect_default().await?;
    println!("✓ Connected to workspace\n");

    // 2. Get all sessions
    let sessions = client.sessions().list(SessionFilter::all())?;
    if sessions.is_empty() {
        println!("No sessions found. Start an agent session first.");
        return Ok(());
    }

    println!("Analyzing {} sessions...\n", sessions.len());

    // 3. Collect Execute command statistics per provider
    let mut provider_stats: HashMap<String, ProviderStats> = HashMap::new();
    let mut total_execute = 0;

    for session_summary in &sessions {
        let session_handle = client.sessions().get(&session_summary.id)?;
        let provider = &session_summary.provider;

        // Get or create stats for this provider
        let stats = provider_stats.entry(provider.clone()).or_default();

        // Extract tool calls from assembled session
        if let Ok(session) = session_handle.assemble() {
            for turn in &session.turns {
                for step in &turn.steps {
                    for tool_exec in &step.tools {
                        let call = &tool_exec.call.content;

                        // Only process Execute payloads
                        if let ToolCallPayload::Execute { arguments, .. } = call {
                            if let Some(command) = arguments.command() {
                                total_execute += 1;
                                stats.total_execute += 1;

                                let intent = ExecuteIntent::classify(command);
                                *stats.intent_counts.entry(intent).or_insert(0) += 1;

                                // Store examples (limit to 3 per intent)
                                let examples = stats.intent_examples.entry(intent).or_default();
                                if examples.len() < 3 {
                                    examples.push(command.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. Display statistics
    println!("Total Execute commands: {}\n", total_execute);

    // Sort providers by execute count
    let mut providers: Vec<_> = provider_stats.iter().collect();
    providers.sort_by(|a, b| b.1.total_execute.cmp(&a.1.total_execute));

    // Display statistics for each provider
    for (provider_name, stats) in providers {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!(
            "Provider: {} (× {} Execute commands)",
            provider_name, stats.total_execute
        );
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();

        // Intent breakdown
        println!("  Execute Intent Breakdown:");
        let mut intents: Vec<_> = stats.intent_counts.iter().collect();
        intents.sort_by(|a, b| b.1.cmp(a.1));

        for (intent, count) in intents {
            let percentage = (*count as f64 / stats.total_execute as f64) * 100.0;
            println!("    {} (× {}, {:.1}%)", intent, count, percentage);

            // Show examples
            if let Some(examples) = stats.intent_examples.get(intent) {
                for example in examples {
                    let truncated = if example.len() > 80 {
                        format!("{}...", &example[..77])
                    } else {
                        example.clone()
                    };
                    println!("      - {}", truncated);
                }
            }
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_sed_read() {
        assert_eq!(
            ExecuteIntent::classify("sed -n 1,200p packages/extension-inspector/src/App.tsx"),
            ExecuteIntent::Read
        );
        assert_eq!(
            ExecuteIntent::classify("sed -n '1,160p' docs/provider_tool_normalization.md"),
            ExecuteIntent::Read
        );
    }

    #[test]
    fn test_classify_sed_write() {
        assert_eq!(
            ExecuteIntent::classify("sed -i 's/foo/bar/g' file.txt"),
            ExecuteIntent::Write
        );
        assert_eq!(
            ExecuteIntent::classify("sed --in-place 's/foo/bar/g' file.txt"),
            ExecuteIntent::Write
        );
    }

    #[test]
    fn test_classify_bash_wrapped_read() {
        assert_eq!(
            ExecuteIntent::classify("bash -lc cat Cargo.toml"),
            ExecuteIntent::Read
        );
        assert_eq!(
            ExecuteIntent::classify("bash -lc sed -n '1,200p' src/main.rs"),
            ExecuteIntent::Read
        );
    }

    #[test]
    fn test_classify_ls() {
        assert_eq!(ExecuteIntent::classify("ls"), ExecuteIntent::Read);
        assert_eq!(ExecuteIntent::classify("ls -la"), ExecuteIntent::Read);
    }

    #[test]
    fn test_classify_cat() {
        assert_eq!(ExecuteIntent::classify("cat file.txt"), ExecuteIntent::Read);
    }

    #[test]
    fn test_classify_grep() {
        assert_eq!(
            ExecuteIntent::classify("grep pattern file.txt"),
            ExecuteIntent::Read
        );
    }

    #[test]
    fn test_classify_git() {
        assert_eq!(ExecuteIntent::classify("git status"), ExecuteIntent::Git);
        assert_eq!(ExecuteIntent::classify("git commit -m 'message'"), ExecuteIntent::Git);
    }

    #[test]
    fn test_classify_test() {
        assert_eq!(ExecuteIntent::classify("cargo test"), ExecuteIntent::Test);
        assert_eq!(ExecuteIntent::classify("npm test"), ExecuteIntent::Test);
    }

    #[test]
    fn test_classify_build() {
        assert_eq!(ExecuteIntent::classify("cargo build"), ExecuteIntent::Build);
        assert_eq!(ExecuteIntent::classify("npm build"), ExecuteIntent::Build);
    }
}
