use crate::model::*;
use crate::normalize::{claude, codex, gemini};
use crate::storage::Storage;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
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
        /// Vendor source type
        #[arg(long, value_parser = ["claude", "codex", "gemini"])]
        source: String,

        /// Root directory of vendor logs
        #[arg(long)]
        root: PathBuf,

        /// Project root path (optional override)
        #[arg(long)]
        project_root: Option<String>,

        /// Session ID prefix
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
            let events = import_vendor_logs(&source, &root, project_root.as_deref(), session_id_prefix.as_deref())?;

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
    }

    Ok(())
}

fn import_vendor_logs(
    source: &str,
    root: &PathBuf,
    project_root_override: Option<&str>,
    session_id_prefix: Option<&str>,
) -> Result<Vec<AgentEventV1>> {
    let mut all_events = Vec::new();

    match source {
        "claude" => {
            for entry in WalkDir::new(root)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() && entry.path().extension().map_or(false, |e| e == "jsonl") {
                    let events = claude::normalize_claude_file(entry.path(), project_root_override)?;
                    all_events.extend(events);
                }
            }
        }
        "codex" => {
            for entry in WalkDir::new(root)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() && entry.path().extension().map_or(false, |e| e == "jsonl") {
                    // Remove .jsonl extension from filename to use as session_id
                    let filename = entry.file_name().to_string_lossy();
                    let session_id_base = if filename.ends_with(".jsonl") {
                        &filename[..filename.len() - 6]
                    } else {
                        filename.as_ref()
                    };

                    let session_id = session_id_prefix
                        .map(|p| format!("{}{}", p, session_id_base))
                        .unwrap_or_else(|| session_id_base.to_string());

                    let events = codex::normalize_codex_file(entry.path(), &session_id, project_root_override)?;
                    all_events.extend(events);
                }
            }
        }
        "gemini" => {
            for entry in WalkDir::new(root)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() && entry.path().extension().map_or(false, |e| e == "json") {
                    let events = gemini::normalize_gemini_file(entry.path())?;
                    all_events.extend(events);
                }
            }
        }
        _ => anyhow::bail!("Unknown source: {}", source),
    }

    Ok(all_events)
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
