use crate::error::Result;
use crate::storage;
use std::path::PathBuf;

pub fn cmd_export(
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
