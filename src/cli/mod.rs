use crate::error::Result;
use crate::model::{Agent, Event};
use crate::storage;
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "agtrace")]
#[command(about = "Unify session histories from AI coding agents", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all executions (reads directly from agent directories)
    List {
        /// Filter by agent type
        #[arg(long)]
        agent: Option<String>,

        /// Custom path to read from
        #[arg(long)]
        path: Option<PathBuf>,

        /// Filter by project path
        #[arg(long)]
        project: Option<PathBuf>,

        /// Filter by date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,

        /// Show all executions (default: 10)
        #[arg(long)]
        all: bool,

        /// Number of executions to show (default: 10)
        #[arg(long)]
        limit: Option<usize>,
    },

    /// Find and show details of an execution by ID (searches all agents)
    Find {
        /// Execution ID
        id: String,

        /// Include event timeline
        #[arg(long)]
        events: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show details of a specific execution
    Show {
        /// Agent type (claude-code or codex)
        agent: String,

        /// Execution ID
        id: String,

        /// Custom path to read from
        #[arg(long)]
        path: Option<PathBuf>,

        /// Include event timeline
        #[arg(long)]
        events: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show statistics (computed on-the-fly from agent directories)
    Stats {
        /// Filter by agent type
        #[arg(long)]
        agent: Option<String>,

        /// Custom path to read from
        #[arg(long)]
        path: Option<PathBuf>,

        /// Group by agent
        #[arg(long)]
        by_agent: bool,

        /// Group by project
        #[arg(long)]
        by_project: bool,

        /// Group by day
        #[arg(long)]
        by_day: bool,
    },

    /// Export executions (reads directly and exports)
    Export {
        /// Agent type (claude-code or codex) - required if id is specified
        agent: Option<String>,

        /// Execution ID (optional, use --all to export all)
        id: Option<String>,

        /// Export all executions
        #[arg(long)]
        all: bool,

        /// Custom path to read from
        #[arg(long)]
        path: Option<PathBuf>,

        /// Output format
        #[arg(long, default_value = "json")]
        format: String,
    },
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::List {
            agent,
            path,
            project,
            since,
            all,
            limit,
        } => cmd_list(agent, path, project, since, all, limit),
        Commands::Find { id, events, json } => cmd_find(&id, events, json),
        Commands::Show {
            agent,
            id,
            path,
            events,
            json,
        } => cmd_show(&agent, &id, path, events, json),
        Commands::Stats {
            agent,
            path,
            by_agent,
            by_project,
            by_day,
        } => cmd_stats(agent, path, by_agent, by_project, by_day),
        Commands::Export {
            agent,
            id,
            all,
            path,
            format,
        } => cmd_export(agent, id, all, path, &format),
    }
}

// Helper functions for compact formatting
fn format_project_short(path: &Path) -> String {
    // /Users/zawakin/go/src/github.com/lanegrid/lanegrid
    // → lanegrid/lanegrid
    path.iter()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|s| s.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}h{}m", seconds / 3600, (seconds % 3600) / 60)
    }
}

fn format_date_short(dt: &DateTime<Utc>) -> String {
    dt.format("%b %d").to_string() // "Nov 22"
}

fn format_id_short(id: &str) -> String {
    // Take first 8 characters of ID
    if id.len() > 8 {
        id[..8].to_string()
    } else {
        id.to_string()
    }
}

fn format_number(n: u64) -> String {
    // Format number with commas: 12345 -> "12,345"
    let s = n.to_string();
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

fn cmd_list(
    agent_filter: Option<String>,
    custom_path: Option<PathBuf>,
    project_filter: Option<PathBuf>,
    since_filter: Option<String>,
    show_all: bool,
    limit: Option<usize>,
) -> Result<()> {
    let mut executions = if let Some(agent) = &agent_filter {
        match agent.as_str() {
            "claude-code" => storage::list_claude_code_executions(custom_path)?,
            "codex" => storage::list_codex_executions(custom_path)?,
            _ => {
                eprintln!(
                    "Error: Unknown agent '{}'. Use 'claude-code' or 'codex'",
                    agent
                );
                return Ok(());
            }
        }
    } else {
        storage::list_all_executions()?
    };

    let total_count = executions.len();

    // Apply filters
    if let Some(project) = project_filter {
        let project = project.canonicalize().unwrap_or(project);
        executions.retain(|e| {
            e.project_path
                .canonicalize()
                .map(|p| p == project)
                .unwrap_or(false)
        });
    }

    if let Some(since) = since_filter {
        if let Ok(since_date) = chrono::NaiveDate::parse_from_str(&since, "%Y-%m-%d") {
            let since_datetime = since_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
            executions.retain(|e| e.started_at >= since_datetime);
        }
    }

    let filtered_count = executions.len();

    // Apply limit (default: 10 unless --all is specified)
    let display_limit = if show_all {
        filtered_count
    } else {
        limit.unwrap_or(10)
    };

    let showing_count = display_limit.min(filtered_count);
    executions.truncate(display_limit);

    // Print header
    if filtered_count == 0 {
        println!("No executions found.");
        return Ok(());
    }

    println!();
    if showing_count < filtered_count {
        println!(
            "Recent executions (showing {} of {}):",
            showing_count, total_count
        );
    } else {
        println!("Recent executions ({}):", filtered_count);
    }
    println!();

    // Print table header
    println!(
        "{:<10} {:<12} {:<25} {:<8} {}",
        "ID", "Agent", "Project", "Duration", "Date"
    );
    println!("{}", "─".repeat(80));

    // Print executions in compact format
    for exec in &executions {
        let agent_name = match &exec.agent {
            Agent::ClaudeCode { .. } => "claude-code",
            Agent::Codex { .. } => "codex",
        };

        let duration = exec
            .metrics
            .duration_seconds
            .map(|s| format_duration(s))
            .unwrap_or_else(|| "?".to_string());

        let project = format_project_short(&exec.project_path);
        let id = format_id_short(&exec.id);
        let date = format_date_short(&exec.started_at);

        println!(
            "{:<10} {:<12} {:<25} {:<8} {}",
            id, agent_name, project, duration, date
        );
    }

    println!();

    // Print footer with hints
    if !show_all && showing_count < filtered_count {
        println!(
            "Use --all to see all {} executions, or --limit N to change count.",
            filtered_count
        );
    }

    // Print summary if there's at least one execution with a summary
    if let Some(exec) = executions.first() {
        if !exec.summaries.is_empty() {
            println!();
            println!("Latest session summary:");
            println!("  {}", exec.summaries[0]);
        }
    }

    Ok(())
}

fn cmd_find(id: &str, show_events: bool, as_json: bool) -> Result<()> {
    // Try exact match first
    let execution = match storage::find_execution(id) {
        Ok(exec) => exec,
        Err(_) => {
            // If exact match fails, try prefix match
            let all_executions = storage::list_all_executions()?;
            let matches: Vec<_> = all_executions
                .into_iter()
                .filter(|e| e.id.starts_with(id))
                .collect();

            match matches.len() {
                0 => return Err(crate::error::Error::ExecutionNotFound(id.to_string())),
                1 => matches.into_iter().next().unwrap(),
                _ => {
                    eprintln!(
                        "Error: Multiple executions match '{}'. Please provide more characters:",
                        id
                    );
                    for exec in matches.iter().take(10) {
                        eprintln!("  {}", exec.id);
                    }
                    return Ok(());
                }
            }
        }
    };

    if as_json {
        let json = serde_json::to_string_pretty(&execution)?;
        println!("{}", json);
        return Ok(());
    }

    // Print compact summary format
    println!();
    println!("Session: {}", execution.id);
    println!();

    // Agent and project info
    let agent_name = match &execution.agent {
        Agent::ClaudeCode { model, .. } => format!("Claude Code ({})", model),
        Agent::Codex { model } => format!("Codex ({})", model),
    };

    let project = format_project_short(&execution.project_path);
    let branch_info = execution
        .git_branch
        .as_ref()
        .map(|b| format!(" ({})", b))
        .unwrap_or_default();

    println!("Agent:    {}", agent_name);
    println!("Project:  {}{}", project, branch_info);

    // Duration info
    if let (Some(duration), Some(ended)) = (execution.metrics.duration_seconds, execution.ended_at)
    {
        let duration_str = if duration < 60 {
            format_duration(duration)
        } else {
            let minutes = duration / 60;
            format!("{} minutes", minutes)
        };
        let start_time = execution.started_at.format("%b %d, %H:%M");
        let end_time = ended.format("%H:%M");
        println!("Duration: {} ({} - {})", duration_str, start_time, end_time);
    } else {
        println!("Started:  {}", execution.started_at.format("%b %d, %H:%M"));
    }
    println!();

    // Summary (if available)
    if !execution.summaries.is_empty() {
        println!("Summary:");
        for summary in &execution.summaries {
            println!("  {}", summary);
        }
        println!();
    }

    // Activity summary
    println!("Activity:");
    println!(
        "  User messages:     {}",
        execution.metrics.user_message_count
    );

    // Tool usage breakdown (compact)
    if !execution.metrics.tool_calls_by_name.is_empty() {
        let mut tools: Vec<_> = execution.metrics.tool_calls_by_name.iter().collect();
        tools.sort_by(|a, b| b.1.cmp(a.1));
        let tool_summary: Vec<String> = tools
            .iter()
            .take(5)
            .map(|(name, count)| format!("{}: {}", name, count))
            .collect();
        println!(
            "  Tool calls:        {} ({})",
            execution.metrics.tool_call_count,
            tool_summary.join(", ")
        );
    } else {
        println!("  Tool calls:        {}", execution.metrics.tool_call_count);
    }

    // Token usage
    println!(
        "  Tokens:            {} in / {} out",
        format_number(execution.metrics.input_tokens),
        format_number(execution.metrics.output_tokens)
    );

    if execution.metrics.cache_read_tokens > 0 {
        println!(
            "                     {} cache read",
            format_number(execution.metrics.cache_read_tokens)
        );
    }

    println!();

    // Event timeline hint
    if show_events {
        println!("Events ({}):", execution.events.len());
        println!();

        // Show limited events by default
        let event_limit = 20;
        let events_to_show = execution.events.len().min(event_limit);

        for (i, event) in execution.events.iter().take(events_to_show).enumerate() {
            print_event(i, event);
        }

        if execution.events.len() > event_limit {
            println!();
            println!(
                "... and {} more events",
                execution.events.len() - event_limit
            );
            println!("Use --json to see full event timeline");
        }
    } else {
        println!("Use --events to see event timeline.");
    }

    Ok(())
}

fn cmd_show(
    agent: &str,
    id: &str,
    custom_path: Option<PathBuf>,
    show_events: bool,
    as_json: bool,
) -> Result<()> {
    let execution = storage::find_execution_by_agent(agent, id, custom_path)?;

    if as_json {
        let json = serde_json::to_string_pretty(&execution)?;
        println!("{}", json);
        return Ok(());
    }

    // Print compact summary format
    println!();
    println!("Session: {}", execution.id);
    println!();

    // Agent and project info
    let agent_name = match &execution.agent {
        Agent::ClaudeCode { model, .. } => format!("Claude Code ({})", model),
        Agent::Codex { model } => format!("Codex ({})", model),
    };

    let project = format_project_short(&execution.project_path);
    let branch_info = execution
        .git_branch
        .as_ref()
        .map(|b| format!(" ({})", b))
        .unwrap_or_default();

    println!("Agent:    {}", agent_name);
    println!("Project:  {}{}", project, branch_info);

    // Duration info
    if let (Some(duration), Some(ended)) = (execution.metrics.duration_seconds, execution.ended_at)
    {
        let duration_str = if duration < 60 {
            format_duration(duration)
        } else {
            let minutes = duration / 60;
            format!("{} minutes", minutes)
        };
        let start_time = execution.started_at.format("%b %d, %H:%M");
        let end_time = ended.format("%H:%M");
        println!("Duration: {} ({} - {})", duration_str, start_time, end_time);
    } else {
        println!("Started:  {}", execution.started_at.format("%b %d, %H:%M"));
    }
    println!();

    // Summary (if available)
    if !execution.summaries.is_empty() {
        println!("Summary:");
        for summary in &execution.summaries {
            println!("  {}", summary);
        }
        println!();
    }

    // Activity summary
    println!("Activity:");
    println!(
        "  User messages:     {}",
        execution.metrics.user_message_count
    );

    // Tool usage breakdown (compact)
    if !execution.metrics.tool_calls_by_name.is_empty() {
        let mut tools: Vec<_> = execution.metrics.tool_calls_by_name.iter().collect();
        tools.sort_by(|a, b| b.1.cmp(a.1));
        let tool_summary: Vec<String> = tools
            .iter()
            .take(5)
            .map(|(name, count)| format!("{}: {}", name, count))
            .collect();
        println!(
            "  Tool calls:        {} ({})",
            execution.metrics.tool_call_count,
            tool_summary.join(", ")
        );
    } else {
        println!("  Tool calls:        {}", execution.metrics.tool_call_count);
    }

    // Token usage
    println!(
        "  Tokens:            {} in / {} out",
        format_number(execution.metrics.input_tokens),
        format_number(execution.metrics.output_tokens)
    );

    if execution.metrics.cache_read_tokens > 0 {
        println!(
            "                     {} cache read",
            format_number(execution.metrics.cache_read_tokens)
        );
    }

    println!();

    // Event timeline hint
    if show_events {
        println!("Events ({}):", execution.events.len());
        println!();

        // Show limited events by default
        let event_limit = 20;
        let events_to_show = execution.events.len().min(event_limit);

        for (i, event) in execution.events.iter().take(events_to_show).enumerate() {
            print_event(i, event);
        }

        if execution.events.len() > event_limit {
            println!();
            println!(
                "... and {} more events",
                execution.events.len() - event_limit
            );
            println!("Use --json to see full event timeline");
        }
    } else {
        println!("Use --events to see event timeline.");
    }

    Ok(())
}

fn print_event(index: usize, event: &Event) {
    match event {
        Event::UserMessage { timestamp, .. } => {
            println!("  [{}] {} - User message", index, timestamp);
        }
        Event::AssistantMessage { timestamp, .. } => {
            println!("  [{}] {} - Assistant message", index, timestamp);
        }
        Event::Thinking {
            duration_ms,
            timestamp,
            ..
        } => {
            let duration = duration_ms
                .map(|ms| format!(" ({}ms)", ms))
                .unwrap_or_default();
            println!("  [{}] {} - Thinking{}", index, timestamp, duration);
        }
        Event::ToolCall {
            name, timestamp, ..
        } => {
            println!("  [{}] {} - Tool call: {}", index, timestamp, name);
        }
        Event::ToolResult { timestamp, .. } => {
            println!("  [{}] {} - Tool result", index, timestamp);
        }
        Event::FileSnapshot { timestamp, .. } => {
            println!("  [{}] {} - File snapshot", index, timestamp);
        }
    }
}

fn cmd_stats(
    agent_filter: Option<String>,
    custom_path: Option<PathBuf>,
    by_agent: bool,
    by_project: bool,
    by_day: bool,
) -> Result<()> {
    let executions = if let Some(agent) = &agent_filter {
        match agent.as_str() {
            "claude-code" => storage::list_claude_code_executions(custom_path)?,
            "codex" => storage::list_codex_executions(custom_path)?,
            _ => {
                eprintln!(
                    "Error: Unknown agent '{}'. Use 'claude-code' or 'codex'",
                    agent
                );
                return Ok(());
            }
        }
    } else {
        storage::list_all_executions()?
    };

    if executions.is_empty() {
        println!("No executions found");
        return Ok(());
    }

    println!("Overall Statistics:");
    println!("  Total executions: {}", executions.len());
    println!();

    let total_tokens: u64 = executions
        .iter()
        .map(|e| e.metrics.input_tokens + e.metrics.output_tokens)
        .sum();
    let total_tool_calls: u32 = executions.iter().map(|e| e.metrics.tool_call_count).sum();

    println!("  Total tokens: {}", total_tokens);
    println!("  Total tool calls: {}", total_tool_calls);
    println!();

    if by_agent {
        println!("By Agent:");
        let mut agent_stats: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for exec in &executions {
            let agent_name = match &exec.agent {
                Agent::ClaudeCode { .. } => "claude-code",
                Agent::Codex { .. } => "codex",
            };
            *agent_stats.entry(agent_name.to_string()).or_insert(0) += 1;
        }
        for (agent, count) in agent_stats {
            println!("  {}: {}", agent, count);
        }
        println!();
    }

    if by_project {
        println!("By Project:");
        let mut project_stats: std::collections::HashMap<PathBuf, usize> =
            std::collections::HashMap::new();
        for exec in &executions {
            *project_stats.entry(exec.project_path.clone()).or_insert(0) += 1;
        }
        for (project, count) in project_stats {
            println!("  {}: {}", project.display(), count);
        }
        println!();
    }

    if by_day {
        println!("By Day:");
        let mut day_stats: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for exec in &executions {
            let day = exec.started_at.format("%Y-%m-%d").to_string();
            *day_stats.entry(day).or_insert(0) += 1;
        }
        let mut days: Vec<_> = day_stats.into_iter().collect();
        days.sort_by(|a, b| a.0.cmp(&b.0));
        for (day, count) in days {
            println!("  {}: {}", day, count);
        }
        println!();
    }

    Ok(())
}

fn cmd_export(
    agent: Option<String>,
    id: Option<String>,
    all: bool,
    custom_path: Option<PathBuf>,
    format: &str,
) -> Result<()> {
    if format != "json" && format != "jsonl" {
        eprintln!("Error: Only 'json' and 'jsonl' formats are supported");
        return Ok(());
    }

    if all {
        let executions = if let Some(agent) = agent {
            match agent.as_str() {
                "claude-code" => storage::list_claude_code_executions(custom_path)?,
                "codex" => storage::list_codex_executions(custom_path)?,
                _ => {
                    eprintln!(
                        "Error: Unknown agent '{}'. Use 'claude-code' or 'codex'",
                        agent
                    );
                    return Ok(());
                }
            }
        } else {
            storage::list_all_executions()?
        };

        if format == "jsonl" {
            for exec in executions {
                let json = serde_json::to_string(&exec)?;
                println!("{}", json);
            }
        } else {
            let json = serde_json::to_string_pretty(&executions)?;
            println!("{}", json);
        }
    } else if let (Some(agent), Some(id)) = (agent, id) {
        let execution = storage::find_execution_by_agent(&agent, &id, custom_path)?;
        let json = serde_json::to_string_pretty(&execution)?;
        println!("{}", json);
    } else {
        eprintln!("Error: Must specify either --all or provide both agent and execution ID");
    }

    Ok(())
}
