use crate::model::AgentEventV1;
use anyhow::{Context, Result};
use std::path::Path;

use super::mapper::normalize_gemini_session;
use super::schema::GeminiSession;

/// Parse Gemini CLI JSON file and normalize to AgentEventV1
pub fn normalize_gemini_file(path: &Path) -> Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Gemini file: {}", path.display()))?;

    let session: GeminiSession = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse Gemini JSON: {}", path.display()))?;

    Ok(normalize_gemini_session(&session))
}

/// Extract projectHash from a Gemini logs.json file
pub fn extract_project_hash_from_gemini_file(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let session: GeminiSession = serde_json::from_str(&text).ok()?;
    Some(session.project_hash)
}

#[derive(Debug)]
pub struct GeminiHeader {
    pub session_id: Option<String>,
    pub timestamp: Option<String>,
    pub snippet: Option<String>,
}

/// Extract header information from Gemini file (for scanning)
pub fn extract_gemini_header(path: &Path) -> Result<GeminiHeader> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Gemini file: {}", path.display()))?;

    let session: GeminiSession = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse Gemini JSON: {}", path.display()))?;

    let snippet = session.messages.iter()
        .find_map(|msg| {
            use super::schema::GeminiMessage;
            match msg {
                GeminiMessage::User(user_msg) => Some(user_msg.content.clone()),
                _ => None,
            }
        });

    Ok(GeminiHeader {
        session_id: Some(session.session_id),
        timestamp: Some(session.start_time),
        snippet,
    })
}
