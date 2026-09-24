use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::sync::Arc;

/// Provider of an agent log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    ClaudeCode,
    Codex,
}

impl Provider {
    /// Canonical provider name (matches the index / CLI provider names).
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::ClaudeCode => "claude_code",
            Provider::Codex => "codex",
        }
    }

    /// Prefix used in [`AgentId`] strings.
    pub fn agent_prefix(&self) -> &'static str {
        match self {
            Provider::ClaudeCode => "claude",
            Provider::Codex => "codex",
        }
    }

    /// Parse a provider name (`claude_code`, `claude`, `codex`).
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "claude_code" | "claude" => Some(Provider::ClaudeCode),
            "codex" => Some(Provider::Codex),
            _ => None,
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Globally unique, stable agent key. Cheap to clone. Serialized as a plain string.
///
/// Grammar:
/// - `claude:<sessionId>` — main or teammate transcript (`<project>/<sessionId>.jsonl`)
/// - `claude:<sessionId>/<agentId>` — async subagent or fork
///   (`<sessionId>/subagents/agent-<agentId>.jsonl`)
/// - `codex:<threadId>` — any Codex thread (root or child)
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AgentId(Arc<str>);

impl AgentId {
    /// Claude main or teammate transcript.
    pub fn claude_session(session_id: &str) -> Self {
        Self(Arc::from(format!("claude:{session_id}")))
    }

    /// Claude async subagent or fork living under `<sessionId>/subagents/`.
    pub fn claude_subagent(session_id: &str, agent_id: &str) -> Self {
        Self(Arc::from(format!("claude:{session_id}/{agent_id}")))
    }

    /// Any Codex thread (root or child).
    pub fn codex_thread(thread_id: &str) -> Self {
        Self(Arc::from(format!("codex:{thread_id}")))
    }

    /// Parse an agent id string. Returns `None` if the provider prefix is unknown
    /// or the native id part is empty.
    pub fn parse(s: &str) -> Option<Self> {
        let (prefix, rest) = s.split_once(':')?;
        if rest.is_empty() {
            return None;
        }
        match prefix {
            "claude" | "codex" => Some(Self(Arc::from(s))),
            _ => None,
        }
    }

    pub fn provider(&self) -> Provider {
        if self.0.starts_with("codex:") {
            Provider::Codex
        } else {
            Provider::ClaudeCode
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Native id part after the provider prefix (`<sessionId>[/<agentId>]` or `<threadId>`).
    fn native(&self) -> &str {
        self.0.split_once(':').map(|(_, rest)| rest).unwrap_or("")
    }

    /// Native session / thread id of the file owner.
    ///
    /// Claude: the `sessionId` (for subagents: the parent session); Codex: the thread id.
    pub fn native_session_id(&self) -> &str {
        let native = self.native();
        match self.provider() {
            Provider::ClaudeCode => native.split_once('/').map(|(s, _)| s).unwrap_or(native),
            Provider::Codex => native,
        }
    }

    /// Native per-agent id for Claude subagents (`claude:<sid>/<aid>` → `<aid>`).
    pub fn native_agent_id(&self) -> Option<&str> {
        match self.provider() {
            Provider::ClaudeCode => self.native().split_once('/').map(|(_, a)| a),
            Provider::Codex => None,
        }
    }

    /// True for Claude subagent / fork ids (`claude:<sid>/<aid>`).
    pub fn is_claude_subagent(&self) -> bool {
        self.native_agent_id().is_some()
    }
}

impl fmt::Debug for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AgentId({})", self.0)
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for AgentId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AgentId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        AgentId::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid AgentId: {s}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_ids() {
        let main = AgentId::claude_session("s1");
        assert_eq!(main.as_str(), "claude:s1");
        assert_eq!(main.provider(), Provider::ClaudeCode);
        assert_eq!(main.native_session_id(), "s1");
        assert_eq!(main.native_agent_id(), None);
        assert!(!main.is_claude_subagent());

        let sub = AgentId::claude_subagent("s1", "a01");
        assert_eq!(sub.as_str(), "claude:s1/a01");
        assert_eq!(sub.native_session_id(), "s1");
        assert_eq!(sub.native_agent_id(), Some("a01"));
        assert!(sub.is_claude_subagent());
    }

    #[test]
    fn codex_ids() {
        let t = AgentId::codex_thread("0190-abc");
        assert_eq!(t.as_str(), "codex:0190-abc");
        assert_eq!(t.provider(), Provider::Codex);
        assert_eq!(t.native_session_id(), "0190-abc");
        assert_eq!(t.native_agent_id(), None);
    }

    #[test]
    fn serde_roundtrip_plain_string() {
        let id = AgentId::claude_subagent("s1", "a01");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"claude:s1/a01\"");
        let back: AgentId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
        assert!(serde_json::from_str::<AgentId>("\"gemini:x\"").is_err());
        assert!(serde_json::from_str::<AgentId>("\"codex:\"").is_err());
    }

    #[test]
    fn provider_names() {
        assert_eq!(Provider::from_name("claude"), Some(Provider::ClaudeCode));
        assert_eq!(
            Provider::from_name("claude_code"),
            Some(Provider::ClaudeCode)
        );
        assert_eq!(Provider::from_name("codex"), Some(Provider::Codex));
        assert_eq!(Provider::from_name("gemini"), None);
        assert_eq!(
            serde_json::to_string(&Provider::ClaudeCode).unwrap(),
            "\"claude_code\""
        );
    }
}
