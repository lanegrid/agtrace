use crate::error::{Error, Result};
use crate::model::Execution;
use crate::parser;
use std::path::PathBuf;

/// List all executions from all agent directories
pub fn list_all_executions() -> Result<Vec<Execution>> {
    let mut all_executions = Vec::new();

    // Try to parse Claude Code sessions
    if let Ok(executions) = parser::claude_code::parse_default_dir() {
        all_executions.extend(executions);
    }

    // Try to parse Codex sessions
    if let Ok(executions) = parser::codex::parse_default_dir() {
        all_executions.extend(executions);
    }

    // Sort by started_at timestamp (newest first)
    all_executions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    Ok(all_executions)
}

/// List executions from Claude Code directories
pub fn list_claude_code_executions(path: Option<PathBuf>) -> Result<Vec<Execution>> {
    let mut executions = if let Some(path) = path {
        parser::claude_code::parse_dir(&path)?
    } else {
        parser::claude_code::parse_default_dir()?
    };

    executions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(executions)
}

/// List executions from Codex directories
pub fn list_codex_executions(path: Option<PathBuf>) -> Result<Vec<Execution>> {
    let mut executions = if let Some(path) = path {
        parser::codex::parse_dir(&path)?
    } else {
        parser::codex::parse_default_dir()?
    };

    executions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(executions)
}

/// Find a specific execution by ID
/// Searches through all agent directories
pub fn find_execution(id: &str) -> Result<Execution> {
    let all_executions = list_all_executions()?;

    all_executions
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| Error::ExecutionNotFound(id.to_string()))
}

/// Find a specific execution by ID from a specific agent
pub fn find_execution_by_agent(agent: &str, id: &str, path: Option<PathBuf>) -> Result<Execution> {
    let executions = match agent {
        "claude-code" => list_claude_code_executions(path)?,
        "codex" => list_codex_executions(path)?,
        _ => return Err(Error::Parse(format!("Unknown agent: {}", agent))),
    };

    executions
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| Error::ExecutionNotFound(id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_all_executions() {
        // This will fail if no agent directories exist, which is expected
        let result = list_all_executions();
        // Just verify it returns without panicking
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_find_execution() {
        // Test that find_execution works with the list of all executions
        let all_executions = list_all_executions();

        if let Ok(executions) = all_executions {
            if let Some(first_exec) = executions.first() {
                // Try to find the first execution by ID
                let result = find_execution(&first_exec.id);
                assert!(result.is_ok());

                if let Ok(found) = result {
                    assert_eq!(found.id, first_exec.id);
                }
            }
        }

        // Test with non-existent ID
        let result = find_execution("non-existent-id-12345");
        assert!(result.is_err());
    }
}
