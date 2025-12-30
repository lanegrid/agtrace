use serde::{Deserialize, Serialize};

/// Stream identifier for multi-stream sessions
/// Enables parallel conversation streams within same session (e.g., background reasoning, subagents)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(tag = "stream_type", content = "stream_data")]
#[serde(rename_all = "snake_case")]
pub enum StreamId {
    /// Main conversation stream (default)
    #[default]
    Main,
    /// Claude sidechain (background agent with specific ID)
    Sidechain { agent_id: String },
    /// Codex subagent (e.g., "review", "test", etc.)
    Subagent { name: String },
}

impl StreamId {
    /// Get string representation for debugging/logging
    pub fn as_str(&self) -> String {
        match self {
            StreamId::Main => "main".to_string(),
            StreamId::Sidechain { agent_id } => format!("sidechain:{}", agent_id),
            StreamId::Subagent { name } => format!("subagent:{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_id_variants() {
        let main_stream = StreamId::Main;
        assert_eq!(main_stream.as_str(), "main");

        let sidechain_stream = StreamId::Sidechain {
            agent_id: "abc123".to_string(),
        };
        assert_eq!(sidechain_stream.as_str(), "sidechain:abc123");

        let subagent_stream = StreamId::Subagent {
            name: "review".to_string(),
        };
        assert_eq!(subagent_stream.as_str(), "subagent:review");
    }
}
