use crate::model::AgentEventV1;
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;

use super::mapper::normalize_gemini_session;

/// Parse Gemini CLI JSON file and normalize to AgentEventV1
pub fn normalize_gemini_file(path: &Path) -> Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Gemini file: {}", path.display()))?;

    let json: Value = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse Gemini JSON: {}", path.display()))?;

    Ok(normalize_gemini_session(&json))
}

/// Extract projectHash from a Gemini logs.json file
pub fn extract_project_hash_from_gemini_file(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;
    json.get("projectHash")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
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

    let json: Value = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse Gemini JSON: {}", path.display()))?;

    let session_id = json.get("sessionId")
        .and_then(|v| v.as_str())
        .map(String::from);

    let timestamp = json.get("timestamp")
        .and_then(|v| v.as_str())
        .or_else(|| json.get("createdAt").and_then(|v| v.as_str()))
        .map(String::from);

    let snippet = json.get("messages")
        .and_then(|msgs| msgs.as_array())
        .and_then(|arr| arr.first())
        .and_then(|msg| msg.get("content"))
        .and_then(|c| c.as_str())
        .map(String::from);

    Ok(GeminiHeader {
        session_id,
        timestamp,
        snippet,
    })
}
