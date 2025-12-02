use crate::cli::OutputFormat;
use crate::error::{Error, Result};
use crate::model::{Agent, Execution};
use crate::storage;
use std::path::PathBuf;

use super::formatters::{format_date_short, format_path_compact, format_task_snippet};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Column {
    Id,
    Agent,
    Path,
    Turns,
    Tools,
    Date,
    Task,
}

impl Column {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "id" => Some(Column::Id),
            "agent" => Some(Column::Agent),
            "path" | "project" => Some(Column::Path),
            "turns" => Some(Column::Turns),
            "tools" => Some(Column::Tools),
            "date" => Some(Column::Date),
            "task" => Some(Column::Task),
            _ => None,
        }
    }

    fn header(&self) -> &'static str {
        match self {
            Column::Id => "ID",
            Column::Agent => "Agent",
            Column::Path => "Path",
            Column::Turns => "Turns",
            Column::Tools => "Tools",
            Column::Date => "Date",
            Column::Task => "Task",
        }
    }

    fn width(&self) -> usize {
        match self {
            Column::Id => 36, // UUID length
            Column::Agent => 12,
            Column::Path => 50, // Longer for full paths
            Column::Turns => 6,
            Column::Tools => 6,
            Column::Date => 10,
            Column::Task => 50,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_list(
    agent_filter: Option<String>,
    custom_path: Option<PathBuf>,
    project_filter: Option<PathBuf>,
    since_filter: Option<String>,
    until_filter: Option<String>,
    min_duration: Option<u64>,
    max_duration: Option<u64>,
    show_all: bool,
    limit: Option<usize>,
    sort_field: String,
    reverse: bool,
    format: OutputFormat,
    quiet: bool,
    no_header: bool,
    columns_str: Option<String>,
    use_color: bool,
) -> Result<()> {
    let mut executions = if let Some(agent) = &agent_filter {
        match agent.as_str() {
            "claude-code" => storage::list_claude_code_executions(custom_path)?,
            "codex" => storage::list_codex_executions(custom_path)?,
            _ => {
                return Err(Error::UnknownAgent(format!(
                    "Unknown agent '{}'. Use 'claude-code' or 'codex'",
                    agent
                )));
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
            e.working_dir
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

    if let Some(until) = until_filter {
        if let Ok(until_date) = chrono::NaiveDate::parse_from_str(&until, "%Y-%m-%d") {
            let until_datetime = until_date.and_hms_opt(23, 59, 59).unwrap().and_utc();
            executions.retain(|e| e.started_at <= until_datetime);
        }
    }

    if let Some(min_dur) = min_duration {
        executions.retain(|e| e.metrics.duration_seconds.unwrap_or(0) >= min_dur);
    }

    if let Some(max_dur) = max_duration {
        executions.retain(|e| e.metrics.duration_seconds.unwrap_or(0) <= max_dur);
    }

    let filtered_count = executions.len();

    // Apply sorting
    match sort_field.as_str() {
        "started_at" => {
            executions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        }
        "duration" => {
            executions.sort_by(|a, b| {
                b.metrics
                    .duration_seconds
                    .unwrap_or(0)
                    .cmp(&a.metrics.duration_seconds.unwrap_or(0))
            });
        }
        "tokens" => {
            executions.sort_by(|a, b| {
                let a_tokens = a.metrics.input_tokens + a.metrics.output_tokens;
                let b_tokens = b.metrics.input_tokens + b.metrics.output_tokens;
                b_tokens.cmp(&a_tokens)
            });
        }
        _ => {
            return Err(Error::InvalidSortField(format!(
                "Invalid sort field '{}'. Use 'started_at', 'duration', or 'tokens'",
                sort_field
            )));
        }
    }

    if reverse {
        executions.reverse();
    }

    // Apply limit (default: 10 unless --all is specified)
    let display_limit = if show_all {
        filtered_count
    } else {
        limit.unwrap_or(10)
    };

    let showing_count = display_limit.min(filtered_count);
    executions.truncate(display_limit);

    // Parse columns
    let columns = parse_columns(columns_str)?;

    // Output based on format
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&executions)?);
        }
        OutputFormat::Jsonl => {
            for exec in &executions {
                println!("{}", serde_json::to_string(exec)?);
            }
        }
        OutputFormat::Table => {
            output_table(
                &executions,
                showing_count,
                filtered_count,
                total_count,
                quiet,
                no_header,
                show_all,
                &columns,
                use_color,
            );
        }
    }

    Ok(())
}

fn parse_columns(columns_str: Option<String>) -> Result<Vec<Column>> {
    if let Some(cols) = columns_str {
        let mut parsed = Vec::new();
        for col in cols.split(',') {
            match Column::from_str(col) {
                Some(c) => parsed.push(c),
                None => {
                    return Err(Error::Parse(format!(
                        "Unknown column '{}'. Valid columns: id, agent, path, turns, tools, date, task",
                        col
                    )));
                }
            }
        }
        Ok(parsed)
    } else {
        // Default columns
        Ok(vec![
            Column::Id,
            Column::Agent,
            Column::Path,
            Column::Turns,
            Column::Date,
        ])
    }
}

fn output_table(
    executions: &[Execution],
    showing_count: usize,
    filtered_count: usize,
    total_count: usize,
    quiet: bool,
    no_header: bool,
    show_all: bool,
    columns: &[Column],
    use_color: bool,
) {
    use nu_ansi_term::Color;

    if filtered_count == 0 {
        if !quiet {
            eprintln!("No executions found.");
        }
        return;
    }

    // Print header
    if !quiet {
        eprintln!();
        if showing_count < filtered_count {
            eprintln!(
                "Recent executions (showing {} of {} after filters, {} total):",
                showing_count, filtered_count, total_count
            );
        } else {
            eprintln!("Recent executions ({}):", filtered_count);
        }
        eprintln!();
    }

    // Build and print table header
    if !no_header {
        let mut header_parts = Vec::new();
        for col in columns {
            header_parts.push(format!("{:<width$}", col.header(), width = col.width()));
        }
        let header = header_parts.join(" ");

        if use_color {
            println!("{}", Color::Yellow.bold().paint(header));
        } else {
            println!("{}", header);
        }

        let total_width: usize =
            columns.iter().map(|c| c.width()).sum::<usize>() + (columns.len() - 1); // spaces between columns
        println!("{}", "─".repeat(total_width));
    }

    // Print executions
    for (idx, exec) in executions.iter().enumerate() {
        let mut row_parts = Vec::new();

        for col in columns {
            let value = match col {
                Column::Id => format!("{:<width$}", exec.id, width = col.width()),
                Column::Agent => {
                    let agent_name = match &exec.agent {
                        Agent::ClaudeCode { .. } => "claude-code",
                        Agent::Codex { .. } => "codex",
                    };
                    format!("{:<width$}", agent_name, width = col.width())
                }
                Column::Path => {
                    format!(
                        "{:<width$}",
                        format_path_compact(&exec.working_dir, col.width()),
                        width = col.width()
                    )
                }
                Column::Turns => {
                    format!(
                        "{:<width$}",
                        exec.metrics.user_message_count,
                        width = col.width()
                    )
                }
                Column::Tools => {
                    format!(
                        "{:<width$}",
                        exec.metrics.tool_call_count,
                        width = col.width()
                    )
                }
                Column::Date => {
                    format!(
                        "{:<width$}",
                        format_date_short(&exec.started_at),
                        width = col.width()
                    )
                }
                Column::Task => {
                    let task = format_task_snippet(exec, col.width());
                    format!("{:<width$}", task, width = col.width())
                }
            };
            row_parts.push(value);
        }

        let line = row_parts.join(" ");

        // Highlight the first (most recent) execution
        if use_color && idx == 0 {
            println!("{}", Color::White.bold().paint(line));
        } else {
            println!("{}", line);
        }
    }

    if !quiet {
        println!();

        // Print footer with hints
        if !show_all && showing_count < filtered_count {
            eprintln!(
                "Use --all to see all {} executions, or --limit N to change count.",
                filtered_count
            );
        }
    }
}
