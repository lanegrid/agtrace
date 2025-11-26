use crate::error::Result;
use crate::model::Agent;
use crate::storage;
use std::path::PathBuf;

use super::formatters::{format_date_short, format_duration, format_id_short, format_project_short};

pub fn cmd_list(
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
