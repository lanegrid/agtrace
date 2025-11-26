use crate::error::Result;
use crate::model::Agent;
use crate::storage;
use std::path::PathBuf;

pub fn cmd_stats(
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
