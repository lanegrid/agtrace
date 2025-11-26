use crate::cli::OutputFormat;
use crate::error::{Error, Result};
use crate::model::{Agent, Execution};
use crate::storage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct OverallStats {
    total_executions: usize,
    total_tokens: u64,
    total_input_tokens: u64,
    total_output_tokens: u64,
    total_cache_read_tokens: u64,
    total_tool_calls: u32,
    total_duration_seconds: u64,
    avg_tokens_per_execution: u64,
    avg_duration_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct AgentStats {
    agent: String,
    executions: usize,
    total_tokens: u64,
    total_tool_calls: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectStats {
    project: String,
    executions: usize,
    total_tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct DayStats {
    day: String,
    executions: usize,
    total_tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct StatsOutput {
    overall: OverallStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    by_agent: Option<Vec<AgentStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    by_project: Option<Vec<ProjectStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    by_day: Option<Vec<DayStats>>,
}

pub fn cmd_stats(
    agent_filter: Option<String>,
    custom_path: Option<PathBuf>,
    by_agent: bool,
    by_project: bool,
    by_day: bool,
    format: OutputFormat,
    use_color: bool,
) -> Result<()> {
    let executions = if let Some(agent) = &agent_filter {
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

    if executions.is_empty() {
        if !format.is_json() {
            eprintln!("No executions found");
        }
        return Ok(());
    }

    // Compute overall stats
    let total_input_tokens: u64 = executions.iter().map(|e| e.metrics.input_tokens).sum();
    let total_output_tokens: u64 = executions.iter().map(|e| e.metrics.output_tokens).sum();
    let total_cache_read_tokens: u64 = executions.iter().map(|e| e.metrics.cache_read_tokens).sum();
    let total_tokens = total_input_tokens + total_output_tokens;
    let total_tool_calls: u32 = executions.iter().map(|e| e.metrics.tool_call_count).sum();
    let total_duration_seconds: u64 = executions
        .iter()
        .map(|e| e.metrics.duration_seconds.unwrap_or(0))
        .sum();

    let overall = OverallStats {
        total_executions: executions.len(),
        total_tokens,
        total_input_tokens,
        total_output_tokens,
        total_cache_read_tokens,
        total_tool_calls,
        total_duration_seconds,
        avg_tokens_per_execution: if executions.is_empty() {
            0
        } else {
            total_tokens / executions.len() as u64
        },
        avg_duration_seconds: if executions.is_empty() {
            0
        } else {
            total_duration_seconds / executions.len() as u64
        },
    };

    // Compute by_agent stats
    let agent_stats = if by_agent {
        Some(compute_agent_stats(&executions))
    } else {
        None
    };

    // Compute by_project stats
    let project_stats = if by_project {
        Some(compute_project_stats(&executions))
    } else {
        None
    };

    // Compute by_day stats
    let day_stats = if by_day {
        Some(compute_day_stats(&executions))
    } else {
        None
    };

    let stats_output = StatsOutput {
        overall,
        by_agent: agent_stats,
        by_project: project_stats,
        by_day: day_stats,
    };

    // Output based on format
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&stats_output)?);
        }
        OutputFormat::Jsonl => {
            println!("{}", serde_json::to_string(&stats_output)?);
        }
        OutputFormat::Table => {
            print_stats_table(&stats_output, use_color);
        }
    }

    Ok(())
}

fn compute_agent_stats(executions: &[Execution]) -> Vec<AgentStats> {
    let mut stats_map: HashMap<String, (usize, u64, u32)> = HashMap::new();

    for exec in executions {
        let agent_name = match &exec.agent {
            Agent::ClaudeCode { .. } => "claude-code",
            Agent::Codex { .. } => "codex",
        };
        let tokens = exec.metrics.input_tokens + exec.metrics.output_tokens;
        let entry = stats_map.entry(agent_name.to_string()).or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += tokens;
        entry.2 += exec.metrics.tool_call_count;
    }

    let mut stats: Vec<AgentStats> = stats_map
        .into_iter()
        .map(|(agent, (executions, total_tokens, total_tool_calls))| AgentStats {
            agent,
            executions,
            total_tokens,
            total_tool_calls,
        })
        .collect();

    stats.sort_by(|a, b| b.executions.cmp(&a.executions));
    stats
}

fn compute_project_stats(executions: &[Execution]) -> Vec<ProjectStats> {
    let mut stats_map: HashMap<PathBuf, (usize, u64)> = HashMap::new();

    for exec in executions {
        let tokens = exec.metrics.input_tokens + exec.metrics.output_tokens;
        let entry = stats_map
            .entry(exec.project_path.clone())
            .or_insert((0, 0));
        entry.0 += 1;
        entry.1 += tokens;
    }

    let mut stats: Vec<ProjectStats> = stats_map
        .into_iter()
        .map(|(project, (executions, total_tokens))| ProjectStats {
            project: project.display().to_string(),
            executions,
            total_tokens,
        })
        .collect();

    stats.sort_by(|a, b| b.executions.cmp(&a.executions));
    stats
}

fn compute_day_stats(executions: &[Execution]) -> Vec<DayStats> {
    let mut stats_map: HashMap<String, (usize, u64)> = HashMap::new();

    for exec in executions {
        let day = exec.started_at.format("%Y-%m-%d").to_string();
        let tokens = exec.metrics.input_tokens + exec.metrics.output_tokens;
        let entry = stats_map.entry(day).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += tokens;
    }

    let mut stats: Vec<DayStats> = stats_map
        .into_iter()
        .map(|(day, (executions, total_tokens))| DayStats {
            day,
            executions,
            total_tokens,
        })
        .collect();

    stats.sort_by(|a, b| a.day.cmp(&b.day));
    stats
}

fn print_stats_table(stats: &StatsOutput, use_color: bool) {
    use nu_ansi_term::Color;

    let header = |text: &str| {
        if use_color {
            Color::Yellow.bold().paint(text).to_string()
        } else {
            text.to_string()
        }
    };

    // Overall stats
    println!();
    println!("{}", header("Overall Statistics:"));
    println!("  Total executions:     {}", stats.overall.total_executions);
    println!(
        "  Total tokens:          {} ({} in / {} out)",
        format_number(stats.overall.total_tokens),
        format_number(stats.overall.total_input_tokens),
        format_number(stats.overall.total_output_tokens)
    );
    if stats.overall.total_cache_read_tokens > 0 {
        println!(
            "  Cache read tokens:     {}",
            format_number(stats.overall.total_cache_read_tokens)
        );
    }
    println!(
        "  Avg tokens/execution:  {}",
        format_number(stats.overall.avg_tokens_per_execution)
    );
    println!("  Total tool calls:      {}", stats.overall.total_tool_calls);
    println!(
        "  Total duration:        {}",
        format_duration_long(stats.overall.total_duration_seconds)
    );
    println!(
        "  Avg duration:          {}",
        format_duration_long(stats.overall.avg_duration_seconds)
    );
    println!();

    // By agent
    if let Some(agent_stats) = &stats.by_agent {
        println!("{}", header("By Agent:"));
        println!(
            "  {:<15} {:<12} {:<15} {}",
            "Agent", "Executions", "Total Tokens", "Tool Calls"
        );
        println!("  {}", "─".repeat(60));
        for stat in agent_stats {
            println!(
                "  {:<15} {:<12} {:<15} {}",
                stat.agent,
                stat.executions,
                format_number(stat.total_tokens),
                stat.total_tool_calls
            );
        }
        println!();
    }

    // By project
    if let Some(project_stats) = &stats.by_project {
        println!("{}", header("By Project:"));
        println!(
            "  {:<50} {:<12} {}",
            "Project", "Executions", "Total Tokens"
        );
        println!("  {}", "─".repeat(80));
        for stat in project_stats.iter().take(10) {
            let project = if stat.project.len() > 48 {
                format!("...{}", &stat.project[stat.project.len() - 45..])
            } else {
                stat.project.clone()
            };
            println!(
                "  {:<50} {:<12} {}",
                project,
                stat.executions,
                format_number(stat.total_tokens)
            );
        }
        if project_stats.len() > 10 {
            println!("  ... and {} more projects", project_stats.len() - 10);
        }
        println!();
    }

    // By day
    if let Some(day_stats) = &stats.by_day {
        println!("{}", header("By Day:"));
        println!("  {:<12} {:<12} {}", "Day", "Executions", "Total Tokens");
        println!("  {}", "─".repeat(40));
        for stat in day_stats {
            println!(
                "  {:<12} {:<12} {}",
                stat.day,
                stat.executions,
                format_number(stat.total_tokens)
            );
        }
        println!();
    }
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn format_duration_long(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        let mins = seconds / 60;
        let secs = seconds % 60;
        format!("{}m {}s", mins, secs)
    } else {
        let hours = seconds / 3600;
        let mins = (seconds % 3600) / 60;
        format!("{}h {}m", hours, mins)
    }
}
