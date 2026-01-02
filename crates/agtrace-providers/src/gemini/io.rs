use crate::{Error, Result};
use std::path::Path;

use super::parser::normalize_gemini_session;
use super::schema::GeminiSession;

/// Parse Gemini CLI JSON file and normalize to AgentEvent
pub fn normalize_gemini_file(path: &Path) -> Result<Vec<agtrace_types::AgentEvent>> {
    let text = std::fs::read_to_string(path)?;

    // Parse as Value to preserve original JSON
    let raw_value: serde_json::Value = serde_json::from_str(&text)?;

    // Try new format (session object) first
    if let Ok(session) = serde_json::from_str::<GeminiSession>(&text) {
        // Extract raw messages array
        let raw_messages = raw_value
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        return Ok(normalize_gemini_session(&session, raw_messages));
    }

    Err(Error::Parse(format!(
        "Gemini normalization only supports session format: {}",
        path.display()
    )))
}

/// Extract projectHash from a Gemini logs.json file
pub fn extract_project_hash_from_gemini_file(path: &Path) -> Option<agtrace_types::ProjectHash> {
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
    let text = std::fs::read_to_string(path)?;

    // Try new format first
    if let Ok(session) = serde_json::from_str::<GeminiSession>(&text) {
        let snippet = session.messages.iter().find_map(|msg| {
            use super::schema::GeminiMessage;
            match msg {
                GeminiMessage::User(user_msg) => {
                    Some(agtrace_types::truncate(&user_msg.content, 200))
                }
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
        let snippet = legacy_messages
            .first()
            .map(|m| agtrace_types::truncate(&m.message, 200));

        return Ok(GeminiHeader {
            session_id,
            timestamp,
            snippet,
        });
    }

    Err(Error::Parse(format!(
        "Failed to parse Gemini file in any known format: {}",
        path.display()
    )))
}
