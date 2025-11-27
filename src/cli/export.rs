use crate::cli::OutputFormat;
use crate::error::{Error, Result};
use crate::storage;
use std::path::PathBuf;

pub fn cmd_export(
    agent: Option<String>,
    id: Option<String>,
    all: bool,
    custom_path: Option<PathBuf>,
    since_filter: Option<String>,
    project_filter: Option<PathBuf>,
    limit: Option<usize>,
    format: OutputFormat,
) -> Result<()> {
    if all {
        let mut executions = if let Some(agent) = agent {
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

        // Apply limit
        if let Some(limit) = limit {
            executions.truncate(limit);
        }

        // Output based on format
        match format {
            OutputFormat::Jsonl => {
                for exec in executions {
                    println!("{}", serde_json::to_string(&exec)?);
                }
            }
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&executions)?);
            }
            OutputFormat::Table => {
                return Err(Error::Parse(
                    "Table format is not supported for export. Use 'json' or 'jsonl'".to_string(),
                ));
            }
        }
    } else if let (Some(agent), Some(id)) = (agent, id) {
        let execution = storage::find_execution_by_agent(&agent, &id, custom_path)?;
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&execution)?);
            }
            OutputFormat::Jsonl => {
                println!("{}", serde_json::to_string(&execution)?);
            }
            OutputFormat::Table => {
                return Err(Error::Parse(
                    "Table format is not supported for export. Use 'json' or 'jsonl'".to_string(),
                ));
            }
        }
    } else {
        return Err(Error::Parse(
            "Must specify either --all or provide both agent and execution ID".to_string(),
        ));
    }

    Ok(())
}
