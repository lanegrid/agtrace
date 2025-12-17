use anyhow::{Context, Result};
use std::path::Path;

use super::normalize::normalize_gemini_session_v2;
use super::schema::GeminiSession;

/// Parse Gemini CLI JSON file and normalize to v2::AgentEvent
pub fn normalize_gemini_file_v2(path: &Path) -> Result<Vec<agtrace_types::AgentEvent>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read Gemini file: {}", path.display()))?;

    // Parse as Value to preserve original JSON
    let raw_value: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse Gemini file as JSON: {}", path.display()))?;

    // Try new format (session object) first
    if let Ok(session) = serde_json::from_str::<GeminiSession>(&text) {
        // Extract raw messages array
        let raw_messages = raw_value
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        return Ok(normalize_gemini_session_v2(&session, raw_messages));
    }

    anyhow::bail!(
        "Gemini v2 normalization only supports session format: {}",
        path.display()
    )
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

    // Try new format first
    if let Ok(session) = serde_json::from_str::<GeminiSession>(&text) {
        let snippet = session.messages.iter().find_map(|msg| {
            use super::schema::GeminiMessage;
            match msg {
                GeminiMessage::User(user_msg) => Some(user_msg.content.clone()),
                _ => None,
            }
        });

        return Ok(GeminiHeader {
            session_id: Some(session.session_id),
            timestamp: Some(session.start_time),
            snippet,
        });
    }

    // Try legacy format
    if let Ok(legacy_messages) =
        serde_json::from_str::<Vec<super::schema::LegacyGeminiMessage>>(&text)
    {
        let session_id = legacy_messages.first().map(|m| m.session_id.clone());
        let timestamp = legacy_messages.first().map(|m| m.timestamp.clone());
        let snippet = legacy_messages.first().map(|m| m.message.clone());

        return Ok(GeminiHeader {
            session_id,
            timestamp,
            snippet,
        });
    }

    anyhow::bail!(
        "Failed to parse Gemini file in any known format: {}",
        path.display()
    )
}
