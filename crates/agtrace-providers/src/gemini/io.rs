use agtrace_types::AgentEventV1;
use anyhow::{Context, Result};
use std::path::Path;

use super::mapper::normalize_gemini_session;
use super::schema::GeminiSession;

/// Parse Gemini CLI JSON file and normalize to AgentEventV1
pub fn normalize_gemini_file(path: &Path) -> Result<Vec<AgentEventV1>> {
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
            .map(|arr| arr.clone())
            .unwrap_or_default();

        return Ok(normalize_gemini_session(&session, raw_messages));
    }

    // Fallback: Try legacy format (array of messages)
    if let Ok(legacy_messages) =
        serde_json::from_str::<Vec<super::schema::LegacyGeminiMessage>>(&text)
    {
        return normalize_legacy_format(path, legacy_messages, raw_value);
    }

    anyhow::bail!(
        "Failed to parse Gemini file in any known format: {}",
        path.display()
    )
}

/// Convert legacy format to session
fn normalize_legacy_format(
    path: &Path,
    messages: Vec<super::schema::LegacyGeminiMessage>,
    raw_value: serde_json::Value,
) -> Result<Vec<AgentEventV1>> {
    use super::schema::{GeminiMessage, UserMessage};

    // Extract session_id from first message
    let session_id = messages
        .first()
        .map(|m| m.session_id.clone())
        .unwrap_or_else(|| "unknown".to_string());

    // Extract project_hash from file path
    let project_hash = extract_project_hash_from_path(path)?;

    // Convert legacy messages to new format
    let converted_messages: Vec<GeminiMessage> = messages
        .iter()
        .map(|msg| {
            GeminiMessage::User(UserMessage {
                id: msg.message_id.to_string(),
                timestamp: msg.timestamp.clone(),
                content: msg.message.clone(),
            })
        })
        .collect();

    // Create synthetic session
    let session = GeminiSession {
        session_id,
        project_hash,
        start_time: messages
            .first()
            .map(|m| m.timestamp.clone())
            .unwrap_or_default(),
        last_updated: messages
            .last()
            .map(|m| m.timestamp.clone())
            .unwrap_or_default(),
        messages: converted_messages,
    };

    // For legacy format, raw_value is an array of messages
    let raw_messages = if let Some(arr) = raw_value.as_array() {
        arr.clone()
    } else {
        vec![]
    };

    Ok(normalize_gemini_session(&session, raw_messages))
}

/// Extract project hash from Gemini file path
fn extract_project_hash_from_path(path: &Path) -> Result<String> {
    // Path format: ~/.gemini/tmp/<project_hash>/logs.json
    let components: Vec<_> = path.components().collect();
    for i in 0..components.len() {
        if components[i].as_os_str() == "tmp" && i + 1 < components.len() {
            if let Some(hash) = components[i + 1].as_os_str().to_str() {
                return Ok(hash.to_string());
            }
        }
    }
    anyhow::bail!(
        "Could not extract project hash from path: {}",
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
