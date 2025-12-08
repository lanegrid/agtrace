use crate::model::*;
use crate::normalize::{claude, codex, gemini};
use crate::storage::Storage;
use crate::utils::{discover_project_root, paths_equal};
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "agtrace")]
#[command(about = "Normalize and analyze agent behavior logs", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Data directory for storing normalized events
    #[arg(long, default_value = "~/.agtrace", global = true)]
    pub data_dir: String,

    /// Output format
    #[arg(long, value_parser = ["plain", "json"], default_value = "plain", global = true)]
    pub format: String,

    /// Log level
    #[arg(long, value_parser = ["error", "warn", "info", "debug", "trace"], default_value = "info", global = true)]
    pub log_level: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Import vendor logs and normalize them
    Import {
        /// Vendor source type (default: all enabled providers)
        #[arg(long, value_parser = ["claude", "codex", "gemini", "all"], default_value = "all")]
        source: String,

        /// Root directory of vendor logs (overrides config log_root)
        #[arg(long)]
        root: Option<PathBuf>,

        /// Project root path (optional override)
        #[arg(long)]
        project_root: Option<String>,

        /// Session ID prefix (codex only)
        #[arg(long)]
        session_id_prefix: Option<String>,

        /// Dry run (don't write to storage)
        #[arg(long)]
        dry_run: bool,

        /// Output JSONL file path
        #[arg(long)]
        out_jsonl: Option<PathBuf>,
    },

    /// List sessions
    List {
        /// Filter by project hash
        #[arg(long)]
        project_hash: Option<String>,

        /// Filter by source
        #[arg(long, value_parser = ["claude", "codex", "gemini"])]
        source: Option<String>,

        /// Maximum number of sessions to show
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Filter by start time (RFC3339)
        #[arg(long)]
        since: Option<String>,

        /// Filter by end time (RFC3339)
        #[arg(long)]
        until: Option<String>,
    },

    /// Show session details
    Show {
        /// Session ID
        session_id: String,

        /// Filter by event types (comma-separated)
        #[arg(long)]
        event_type: Option<String>,

        /// Hide reasoning events
        #[arg(long)]
        no_reasoning: bool,

        /// Hide tool events
        #[arg(long)]
        no_tool: bool,

        /// Maximum number of events to show
        #[arg(long)]
        limit: Option<usize>,
    },

    /// Find events
    Find {
        /// Session ID
        #[arg(long)]
        session_id: Option<String>,

        /// Project hash
        #[arg(long)]
        project_hash: Option<String>,

        /// Event ID
        #[arg(long)]
        event_id: Option<String>,

        /// Text search query
        #[arg(long)]
        text: Option<String>,

        /// Event type filter
        #[arg(long)]
        event_type: Option<String>,

        /// Maximum results
        #[arg(long, default_value = "50")]
        limit: usize,
    },

    /// Show statistics
    Stats {
        /// Project hash
        #[arg(long)]
        project_hash: Option<String>,

        /// Source filter
        #[arg(long, value_parser = ["claude", "codex", "gemini"])]
        source: Option<String>,

        /// Group by field
        #[arg(long, value_parser = ["project", "session", "source"])]
        group_by: Option<String>,

        /// Since time (RFC3339)
        #[arg(long)]
        since: Option<String>,

        /// Until time (RFC3339)
        #[arg(long)]
        until: Option<String>,
    },

    /// Export events
    Export {
        /// Project hash
        #[arg(long)]
        project_hash: Option<String>,

        /// Session ID
        #[arg(long)]
        session_id: Option<String>,

        /// Source filter
        #[arg(long, value_parser = ["claude", "codex", "gemini"])]
        source: Option<String>,

        /// Event type filter
        #[arg(long)]
        event_type: Option<String>,

        /// Since time (RFC3339)
        #[arg(long)]
        since: Option<String>,

        /// Until time (RFC3339)
        #[arg(long)]
        until: Option<String>,

        /// Output file path
        #[arg(long)]
        out: PathBuf,

        /// Output format
        #[arg(long, value_parser = ["jsonl", "csv"], default_value = "jsonl")]
        format: String,
    },

    /// Manage provider configurations
    Providers {
        #[command(subcommand)]
        command: Option<ProvidersCommand>,
    },

    /// Show project information
    Project {
        /// Project root path (optional override)
        #[arg(long)]
        project_root: Option<String>,
    },

    /// Show project and session diagnostics
    Status {
        /// Project root path (optional override)
        #[arg(long)]
        project_root: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ProvidersCommand {
    /// List all providers
    List,

    /// Detect providers automatically
    Detect,

    /// Set provider configuration
    Set {
        /// Provider name
        provider: String,

        /// Log root directory
        #[arg(long)]
        log_root: PathBuf,

        /// Enable the provider
        #[arg(long)]
        enable: bool,

        /// Disable the provider
        #[arg(long)]
        disable: bool,
    },
}

pub fn run(cli: Cli) -> Result<()> {
    let data_dir = expand_tilde(&cli.data_dir);
    let storage = Storage::new(data_dir);

    match cli.command {
        Commands::Import {
            source,
            root,
            project_root,
            session_id_prefix,
            dry_run,
            out_jsonl,
        } => {
            let events = import_vendor_logs(&source, root.as_ref(), project_root.as_deref(), session_id_prefix.as_deref())?;

            if dry_run {
                println!("Dry run: Would import {} events from {} sessions",
                    events.len(),
                    count_unique_sessions(&events));
            } else {
                storage.save_events(&events)?;
                println!("Imported {} events from {} sessions",
                    events.len(),
                    count_unique_sessions(&events));
            }

            if let Some(out_path) = out_jsonl {
                write_jsonl(&out_path, &events)?;
                println!("Wrote events to {}", out_path.display());
            }
        }

        Commands::List {
            project_hash,
            source,
            limit,
            since: _,
            until: _,
        } => {
            let source_enum = source.as_deref().and_then(parse_source);
            let sessions = storage.list_sessions(project_hash.as_deref(), source_enum, Some(limit))?;

            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&sessions)?);
            } else {
                print_sessions_table(&sessions);
            }
        }

        Commands::Show {
            session_id,
            event_type: _,
            no_reasoning,
            no_tool,
            limit,
        } => {
            let mut events = storage.load_session_events(&session_id)?;

            if no_reasoning {
                events.retain(|e| e.event_type != EventType::Reasoning);
            }

            if no_tool {
                events.retain(|e| e.event_type != EventType::ToolCall && e.event_type != EventType::ToolResult);
            }

            if let Some(lim) = limit {
                events.truncate(lim);
            }

            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&events)?);
            } else {
                print_events_timeline(&events);
            }
        }

        Commands::Find {
            session_id,
            project_hash,
            event_id: _,
            text,
            event_type,
            limit,
        } => {
            let event_type_enum = event_type.as_deref().and_then(parse_event_type);
            let events = storage.find_events(
                session_id.as_deref(),
                project_hash.as_deref(),
                text.as_deref(),
                event_type_enum,
                Some(limit),
            )?;

            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&events)?);
            } else {
                print_events_timeline(&events);
            }
        }

        Commands::Stats {
            project_hash,
            source,
            group_by: _,
            since: _,
            until: _,
        } => {
            let source_enum = source.as_deref().and_then(parse_source);
            let sessions = storage.list_sessions(project_hash.as_deref(), source_enum, None)?;

            print_stats(&sessions);
        }

        Commands::Export {
            project_hash,
            session_id,
            source: _,
            event_type,
            since: _,
            until: _,
            out,
            format,
        } => {
            let event_type_enum = event_type.as_deref().and_then(parse_event_type);
            let events = storage.find_events(
                session_id.as_deref(),
                project_hash.as_deref(),
                None,
                event_type_enum,
                None,
            )?;

            match format.as_str() {
                "jsonl" => write_jsonl(&out, &events)?,
                "csv" => write_csv(&out, &events)?,
                _ => anyhow::bail!("Unsupported format: {}", format),
            }

            println!("Exported {} events to {}", events.len(), out.display());
        }

        Commands::Providers { command } => {
            match command {
                None | Some(ProvidersCommand::List) => {
                    // List all providers
                    let config = crate::config::Config::load()?;

                    if config.providers.is_empty() {
                        println!("No providers configured. Run 'agtrace providers detect' to auto-detect.");
                        return Ok(());
                    }

                    println!("{:<15} {:<10} {}", "PROVIDER", "ENABLED", "LOG_ROOT");
                    println!("{}", "-".repeat(80));

                    for (name, provider_config) in &config.providers {
                        println!("{:<15} {:<10} {}",
                            name,
                            if provider_config.enabled { "yes" } else { "no" },
                            provider_config.log_root.display());
                    }
                }

                Some(ProvidersCommand::Detect) => {
                    let config = crate::config::Config::detect_providers()?;
                    config.save()?;

                    println!("Detected {} provider(s):", config.providers.len());
                    for (name, provider_config) in &config.providers {
                        println!("  {} -> {}", name, provider_config.log_root.display());
                    }
                }

                Some(ProvidersCommand::Set {
                    provider,
                    log_root,
                    enable,
                    disable,
                }) => {
                    if enable && disable {
                        anyhow::bail!("Cannot specify both --enable and --disable");
                    }

                    let mut config = crate::config::Config::load()?;

                    let enabled = if enable {
                        true
                    } else if disable {
                        false
                    } else {
                        // Default to enabled
                        true
                    };

                    config.set_provider(provider.clone(), crate::config::ProviderConfig {
                        enabled,
                        log_root: log_root.clone(),
                    });

                    config.save()?;

                    println!("Set provider '{}': enabled={}, log_root={}",
                        provider, enabled, log_root.display());
                }
            }
        }

        Commands::Project { project_root } => {
            let project_root_path = discover_project_root(project_root.as_deref())?;
            let project_hash = crate::utils::project_hash_from_root(&project_root_path.to_string_lossy());

            println!("Project root: {}", project_root_path.display());
            println!("Project hash: {}", project_hash);
            println!();

            let config = crate::config::Config::load()?;
            println!("Detected providers:");
            for (name, provider_config) in &config.providers {
                println!("  {}: {}, log_root = {}",
                    name,
                    if provider_config.enabled { "enabled" } else { "disabled" },
                    provider_config.log_root.display());
            }
        }

        Commands::Status { project_root } => {
            let project_root_path = discover_project_root(project_root.as_deref())?;
            let project_hash = crate::utils::project_hash_from_root(&project_root_path.to_string_lossy());

            println!("Project root: {}", project_root_path.display());
            println!("Project hash: {}", project_hash);
            println!();

            let config = crate::config::Config::load()?;
            println!("Providers:");

            for (name, provider_config) in &config.providers {
                if !provider_config.enabled {
                    continue;
                }

                println!("  {}:", name);
                println!("    log_root: {}", provider_config.log_root.display());

                // Count total sessions and matching sessions
                let (total, matching) = match name.as_str() {
                    "claude" => count_claude_sessions(&provider_config.log_root, &project_root_path),
                    "codex" => count_codex_sessions(&provider_config.log_root, &project_root_path),
                    "gemini" => count_gemini_sessions(&provider_config.log_root, &project_hash),
                    _ => (0, 0),
                };

                println!("    sessions detected: {}", total);
                println!("    sessions matching this project: {}", matching);
                println!();
            }
        }
    }

    Ok(())
}

fn import_vendor_logs(
    source: &str,
    root: Option<&PathBuf>,
    project_root_override: Option<&str>,
    session_id_prefix: Option<&str>,
) -> Result<Vec<AgentEventV1>> {
    let mut all_events = Vec::new();

    // Handle "all" source - import from all enabled providers
    if source == "all" {
        if root.is_some() {
            anyhow::bail!("Cannot specify --root when using --source=all");
        }

        // Load config to get enabled providers
        let config = crate::config::Config::load()?;
        let enabled_providers = config.enabled_providers();

        if enabled_providers.is_empty() {
            eprintln!("Warning: No enabled providers found in config. Run 'agtrace providers detect' first.");
            return Ok(all_events);
        }

        for (provider_name, provider_config) in enabled_providers {
            println!("Importing from {} (log_root: {})", provider_name, provider_config.log_root.display());
            match provider_name.as_str() {
                "claude" => {
                    let events = import_claude_directory(&provider_config.log_root, project_root_override)?;
                    all_events.extend(events);
                }
                "codex" => {
                    let events = import_codex_directory(&provider_config.log_root, project_root_override, session_id_prefix)?;
                    all_events.extend(events);
                }
                "gemini" => {
                    let events = import_gemini_directory(&provider_config.log_root)?;
                    all_events.extend(events);
                }
                _ => {
                    eprintln!("Warning: Unknown provider '{}', skipping", provider_name);
                }
            }
        }

        return Ok(all_events);
    }

    // Single source mode - root can be specified or use config
    let root_path = if let Some(r) = root {
        r.clone()
    } else {
        // Load from config
        let config = crate::config::Config::load()?;
        if let Some(provider_config) = config.providers.get(source) {
            provider_config.log_root.clone()
        } else {
            anyhow::bail!("Provider '{}' not found in config. Run 'agtrace providers detect' first.", source);
        }
    };

    // Check if root is a file or directory (Section 2.5.4)
    if root_path.is_file() {
        // Process single file
        match source {
            "claude" => {
                let events = claude::normalize_claude_file(&root_path, project_root_override)?;
                all_events.extend(events);
            }
            "codex" => {
                let filename = root_path.file_name()
                    .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?
                    .to_string_lossy();
                let session_id_base = if filename.ends_with(".jsonl") {
                    &filename[..filename.len() - 6]
                } else {
                    filename.as_ref()
                };

                let session_id = session_id_prefix
                    .map(|p| format!("{}{}", p, session_id_base))
                    .unwrap_or_else(|| session_id_base.to_string());

                let events = codex::normalize_codex_file(&root_path, &session_id, project_root_override)?;
                all_events.extend(events);
            }
            "gemini" => {
                let events = gemini::normalize_gemini_file(&root_path)?;
                all_events.extend(events);
            }
            _ => anyhow::bail!("Unknown source: {}", source),
        }
    } else if root_path.is_dir() {
        // Process directory with vendor-specific rules
        match source {
            "claude" => {
                all_events = import_claude_directory(&root_path, project_root_override)?;
            }
            "codex" => {
                all_events = import_codex_directory(&root_path, project_root_override, session_id_prefix)?;
            }
            "gemini" => {
                all_events = import_gemini_directory(&root_path)?;
            }
            _ => anyhow::bail!("Unknown source: {}", source),
        }
    } else {
        anyhow::bail!("Root path does not exist or is not accessible: {}", root_path.display());
    }

    Ok(all_events)
}

// Section 2.5.6: Claude Code directory import with session matching
fn import_claude_directory(
    root: &PathBuf,
    project_root_override: Option<&str>,
) -> Result<Vec<AgentEventV1>> {
    let mut all_events = Vec::new();

    // Determine target project root
    let target_project_root = if let Some(pr) = project_root_override {
        PathBuf::from(pr)
    } else {
        discover_project_root(None)?
    };

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        // Must be .jsonl file
        if !entry.path().extension().map_or(false, |e| e == "jsonl") {
            continue;
        }

        // Best-effort validation: Check first line for type and sessionId fields
        // If validation fails, we still try to process the file (spec 2.5.6)
        let _is_valid = is_valid_claude_file(entry.path());
        // Note: We intentionally ignore the result and try to normalize anyway

        // Session Matching: Extract cwd from file and check if it matches target project
        if let Some(session_cwd) = claude::extract_cwd_from_claude_file(entry.path()) {
            let session_cwd_path = Path::new(&session_cwd);
            if !paths_equal(&target_project_root, session_cwd_path) {
                // Skip this session as it doesn't match the target project
                continue;
            }
        } else {
            // If we can't determine cwd, skip this file
            eprintln!("Warning: Could not extract cwd from {}, skipping", entry.path().display());
            continue;
        }

        // Try to normalize the file. If it fails, skip this file but continue processing others
        match claude::normalize_claude_file(entry.path(), project_root_override) {
            Ok(events) => {
                all_events.extend(events);
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", entry.path().display(), e);
                continue;
            }
        }
    }

    Ok(all_events)
}

// Section 2.5.5: Codex directory import with session matching
fn import_codex_directory(
    root: &PathBuf,
    project_root_override: Option<&str>,
    session_id_prefix: Option<&str>,
) -> Result<Vec<AgentEventV1>> {
    let mut all_events = Vec::new();

    // Determine target project root
    let target_project_root = if let Some(pr) = project_root_override {
        PathBuf::from(pr)
    } else {
        discover_project_root(None)?
    };

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        // Must be .jsonl file
        if !entry.path().extension().map_or(false, |e| e == "jsonl") {
            continue;
        }

        // Filename must start with "rollout-"
        let filename = entry.file_name().to_string_lossy();
        if !filename.starts_with("rollout-") {
            continue;
        }

        // Session Matching: Extract cwd from file and check if it matches target project
        if let Some(session_cwd) = codex::extract_cwd_from_codex_file(entry.path()) {
            let session_cwd_path = Path::new(&session_cwd);
            if !paths_equal(&target_project_root, session_cwd_path) {
                // Skip this session as it doesn't match the target project
                continue;
            }
        } else {
            // If we can't determine cwd, skip this file
            eprintln!("Warning: Could not extract cwd from {}, skipping", entry.path().display());
            continue;
        }

        // Remove .jsonl extension from filename to use as session_id
        let session_id_base = if filename.ends_with(".jsonl") {
            &filename[..filename.len() - 6]
        } else {
            filename.as_ref()
        };

        let session_id = session_id_prefix
            .map(|p| format!("{}{}", p, session_id_base))
            .unwrap_or_else(|| session_id_base.to_string());

        // Try to normalize the file. If it fails, skip this file but continue processing others
        match codex::normalize_codex_file(entry.path(), &session_id, project_root_override) {
            Ok(events) => {
                all_events.extend(events);
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", entry.path().display(), e);
                continue;
            }
        }
    }

    Ok(all_events)
}

// Section 2.5.7: Gemini CLI directory import with session matching
fn import_gemini_directory(root: &PathBuf) -> Result<Vec<AgentEventV1>> {
    let mut all_events = Vec::new();

    // For Gemini, we need to determine the target project_hash
    // Since Gemini uses projectHash directly, we compute it from project_root
    let project_root = discover_project_root(None)?;
    let target_project_hash = crate::utils::project_hash_from_root(
        &project_root.to_string_lossy()
    );

    // Read direct subdirectories only
    let entries = std::fs::read_dir(root)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        // Directory name must be 64-character hex string
        let dir_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if !is_64_char_hex(dir_name) {
            continue;
        }

        // Must contain logs.json
        let logs_json_path = path.join("logs.json");
        if !logs_json_path.exists() {
            continue;
        }

        // Session Matching: Extract projectHash from logs.json and check if it matches target
        if let Some(session_project_hash) = gemini::extract_project_hash_from_gemini_file(&logs_json_path) {
            if session_project_hash != target_project_hash {
                // Skip this session as it doesn't match the target project
                continue;
            }
        } else {
            // If we can't determine projectHash, skip this directory
            eprintln!("Warning: Could not extract projectHash from {}, skipping", logs_json_path.display());
            continue;
        }

        // Process logs.json
        match gemini::normalize_gemini_file(&logs_json_path) {
            Ok(events) => {
                all_events.extend(events);
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", logs_json_path.display(), e);
                // Continue processing other files
            }
        }

        // Process chats/session-*.json if exists
        let chats_dir = path.join("chats");
        if chats_dir.is_dir() {
            if let Ok(chat_entries) = std::fs::read_dir(&chats_dir) {
                for chat_entry in chat_entries {
                    if let Ok(chat_entry) = chat_entry {
                        let chat_path = chat_entry.path();
                        let chat_filename = chat_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");

                        // Must match session-*.json pattern
                        if chat_path.is_file()
                            && chat_filename.starts_with("session-")
                            && chat_filename.ends_with(".json")
                        {
                            match gemini::normalize_gemini_file(&chat_path) {
                                Ok(events) => {
                                    all_events.extend(events);
                                }
                                Err(e) => {
                                    eprintln!("Warning: Failed to parse {}: {}", chat_path.display(), e);
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(all_events)
}

// Helper: Check if string is 64-character hexadecimal
fn is_64_char_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

// Helper: Validate Claude file by checking first line
fn is_valid_claude_file(path: &std::path::Path) -> Result<bool> {
    use std::io::{BufRead, BufReader};

    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    if let Some(Ok(first_line)) = reader.lines().next() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&first_line) {
            // Check if both "type" and "sessionId" fields exist
            return Ok(json.get("type").is_some() && json.get("sessionId").is_some());
        }
    }

    Ok(false)
}

fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

fn count_unique_sessions(events: &[AgentEventV1]) -> usize {
    let mut sessions = std::collections::HashSet::new();
    for event in events {
        if let Some(sid) = &event.session_id {
            sessions.insert(sid.clone());
        }
    }
    sessions.len()
}

fn parse_source(s: &str) -> Option<Source> {
    match s {
        "claude" => Some(Source::ClaudeCode),
        "codex" => Some(Source::Codex),
        "gemini" => Some(Source::Gemini),
        _ => None,
    }
}

fn parse_event_type(s: &str) -> Option<EventType> {
    match s {
        "user_message" => Some(EventType::UserMessage),
        "assistant_message" => Some(EventType::AssistantMessage),
        "reasoning" => Some(EventType::Reasoning),
        "tool_call" => Some(EventType::ToolCall),
        "tool_result" => Some(EventType::ToolResult),
        _ => None,
    }
}

fn print_sessions_table(sessions: &[SessionSummary]) {
    println!("{:<40} {:<15} {:<20} {:<25} {:<8} {:<8} {:<20}",
        "SESSION ID", "SOURCE", "PROJECT HASH", "START TIME", "EVENTS", "USER MSG", "TOKENS(in/out)");
    println!("{}", "-".repeat(150));

    for session in sessions {
        let source_str = format!("{:?}", session.source).to_lowercase();
        let project_short = if session.project_hash.len() > 20 {
            format!("{}...", &session.project_hash[..17])
        } else {
            session.project_hash.clone()
        };

        println!("{:<40} {:<15} {:<20} {:<25} {:<8} {:<8} {:>9} / {:<9}",
            session.session_id,
            source_str,
            project_short,
            session.start_ts,
            session.event_count,
            session.user_message_count,
            format_number(session.tokens_input_total),
            format_number(session.tokens_output_total));
    }
}

fn print_events_timeline(events: &[AgentEventV1]) {
    for event in events {
        let event_type_str = match event.event_type {
            EventType::UserMessage => "user_message",
            EventType::AssistantMessage => "assistant_message",
            EventType::SystemMessage => "system_message",
            EventType::Reasoning => "reasoning",
            EventType::ToolCall => "tool_call",
            EventType::ToolResult => "tool_result",
            EventType::FileSnapshot => "file_snapshot",
            EventType::SessionSummary => "session_summary",
            EventType::Meta => "meta",
            EventType::Log => "log",
        };
        let role_str = event.role.map(|r| format!("{:?}", r)).unwrap_or_else(|| "".to_string());

        println!("[{}] {:<20} (role={})",
            event.ts,
            event_type_str,
            role_str);

        if let Some(text) = &event.text {
            let preview = if text.len() > 100 {
                format!("{}...", &text[..97])
            } else {
                text.clone()
            };
            println!("  {}", preview);
        }

        if let Some(tool_name) = &event.tool_name {
            println!("  tool: {}", tool_name);
        }

        println!();
    }
}

fn print_stats(sessions: &[SessionSummary]) {
    let total_sessions = sessions.len();
    let total_events: usize = sessions.iter().map(|s| s.event_count).sum();
    let total_user_msgs: usize = sessions.iter().map(|s| s.user_message_count).sum();
    let total_tokens_in: u64 = sessions.iter().map(|s| s.tokens_input_total).sum();
    let total_tokens_out: u64 = sessions.iter().map(|s| s.tokens_output_total).sum();

    println!("OVERALL STATISTICS");
    println!("{}", "=".repeat(60));
    println!("Total Sessions:       {}", total_sessions);
    println!("Total Events:         {}", total_events);
    println!("Total User Messages:  {}", total_user_msgs);
    println!("Total Tokens Input:   {}", format_number(total_tokens_in));
    println!("Total Tokens Output:  {}", format_number(total_tokens_out));
    println!("Total Tokens:         {}", format_number(total_tokens_in + total_tokens_out));
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push('_');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn write_jsonl(path: &PathBuf, events: &[AgentEventV1]) -> Result<()> {
    let mut lines = Vec::new();
    for event in events {
        lines.push(serde_json::to_string(event)?);
    }
    std::fs::write(path, lines.join("\n") + "\n")?;
    Ok(())
}

fn write_csv(path: &PathBuf, events: &[AgentEventV1]) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;

    // Write header
    wtr.write_record(&[
        "ts",
        "source",
        "session_id",
        "event_id",
        "event_type",
        "role",
        "text",
        "tool_name",
        "tokens_input",
        "tokens_output",
    ])?;

    // Write rows
    for event in events {
        wtr.write_record(&[
            &event.ts,
            &format!("{:?}", event.source),
            event.session_id.as_deref().unwrap_or(""),
            event.event_id.as_deref().unwrap_or(""),
            &format!("{:?}", event.event_type),
            event.role.as_ref().map(|r| format!("{:?}", r)).as_deref().unwrap_or(""),
            event.text.as_deref().unwrap_or(""),
            event.tool_name.as_deref().unwrap_or(""),
            event.tokens_input.map(|t| t.to_string()).as_deref().unwrap_or(""),
            event.tokens_output.map(|t| t.to_string()).as_deref().unwrap_or(""),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

// Helper functions for counting sessions in the status command

fn count_claude_sessions(log_root: &PathBuf, project_root: &PathBuf) -> (usize, usize) {
    let mut total = 0;
    let mut matching = 0;

    for entry in WalkDir::new(log_root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        if !entry.path().extension().map_or(false, |e| e == "jsonl") {
            continue;
        }

        total += 1;

        if let Some(cwd) = claude::extract_cwd_from_claude_file(entry.path()) {
            let cwd_path = Path::new(&cwd);
            if paths_equal(project_root, cwd_path) {
                matching += 1;
            }
        }
    }

    (total, matching)
}

fn count_codex_sessions(log_root: &PathBuf, project_root: &PathBuf) -> (usize, usize) {
    let mut total = 0;
    let mut matching = 0;

    for entry in WalkDir::new(log_root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        if !entry.path().extension().map_or(false, |e| e == "jsonl") {
            continue;
        }

        let filename = entry.file_name().to_string_lossy();
        if !filename.starts_with("rollout-") {
            continue;
        }

        total += 1;

        if let Some(cwd) = codex::extract_cwd_from_codex_file(entry.path()) {
            let cwd_path = Path::new(&cwd);
            if paths_equal(project_root, cwd_path) {
                matching += 1;
            }
        }
    }

    (total, matching)
}

fn count_gemini_sessions(log_root: &PathBuf, target_project_hash: &str) -> (usize, usize) {
    let mut total = 0;
    let mut matching = 0;

    if let Ok(entries) = std::fs::read_dir(log_root) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                if !path.is_dir() {
                    continue;
                }

                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if !is_64_char_hex(dir_name) {
                    continue;
                }

                let logs_json_path = path.join("logs.json");
                if !logs_json_path.exists() {
                    continue;
                }

                total += 1;

                if let Some(project_hash) = gemini::extract_project_hash_from_gemini_file(&logs_json_path) {
                    if project_hash == target_project_hash {
                        matching += 1;
                    }
                }
            }
        }
    }

    (total, matching)
}
