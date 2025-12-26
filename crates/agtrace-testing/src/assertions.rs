//! Custom assertions for agtrace-specific validation.
//!
//! Provides high-level assertions that make tests more readable:
//! - Session count validation
//! - Project hash verification
//! - JSON structure checks

use anyhow::{Context, Result};
use serde_json::Value;

/// Assert that JSON output contains expected number of sessions.
pub fn assert_session_count(json: &Value, expected: usize) -> Result<()> {
    let sessions = json["content"]["sessions"]
        .as_array()
        .context("Expected 'content.sessions' array in JSON")?;

    if sessions.len() != expected {
        anyhow::bail!("Expected {} sessions, got {}", expected, sessions.len());
    }

    Ok(())
}

/// Assert that JSON output contains expected number of projects.
pub fn assert_project_count(json: &Value, expected: usize) -> Result<()> {
    let projects = json["content"]["projects"]
        .as_array()
        .context("Expected 'content.projects' array in JSON")?;

    if projects.len() != expected {
        anyhow::bail!("Expected {} projects, got {}", expected, projects.len());
    }

    Ok(())
}

/// Assert that all sessions belong to the specified project hash.
pub fn assert_sessions_belong_to_project(json: &Value, project_hash: &str) -> Result<()> {
    let sessions = json["content"]["sessions"]
        .as_array()
        .context("Expected 'content.sessions' array in JSON")?;

    for (i, session) in sessions.iter().enumerate() {
        let session_project = session["project_hash"]
            .as_str()
            .with_context(|| format!("Session {} missing project_hash", i))?;

        if session_project != project_hash {
            anyhow::bail!(
                "Session {} belongs to project {} but expected {}",
                i,
                session_project,
                project_hash
            );
        }
    }

    Ok(())
}

/// Assert that project list contains specific project hashes.
pub fn assert_projects_contain(json: &Value, expected_hashes: &[&str]) -> Result<()> {
    let projects = json["content"]["projects"]
        .as_array()
        .context("Expected 'content.projects' array in JSON")?;

    let project_hashes: Vec<String> = projects
        .iter()
        .filter_map(|p| p["hash"].as_str().map(String::from))
        .collect();

    for expected in expected_hashes {
        if !project_hashes.contains(&expected.to_string()) {
            anyhow::bail!(
                "Expected project hash {} not found in {:?}",
                expected,
                project_hashes
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_assert_session_count() {
        let json = json!({
            "content": {
                "sessions": [
                    {"id": "1"},
                    {"id": "2"}
                ]
            }
        });

        assert!(assert_session_count(&json, 2).is_ok());
        assert!(assert_session_count(&json, 1).is_err());
    }

    #[test]
    fn test_assert_sessions_belong_to_project() {
        let json = json!({
            "content": {
                "sessions": [
                    {"project_hash": "abc123"},
                    {"project_hash": "abc123"}
                ]
            }
        });

        assert!(assert_sessions_belong_to_project(&json, "abc123").is_ok());
        assert!(assert_sessions_belong_to_project(&json, "xyz789").is_err());
    }
}
