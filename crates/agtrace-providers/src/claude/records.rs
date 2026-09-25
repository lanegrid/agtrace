//! Typed Claude Code (≥ 2.1.24x) records, discriminated by the top-level `type`.
//!
//! Lenient by construction: every field that is not needed to identify a record
//! is optional, unknown record kinds deserialize to [`ClaudeRecord::Unknown`], and
//! modeled-as-ignored kinds to [`ClaudeRecord::Ignored`].

use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

use super::content::{AssistantMessage, UserMessage};

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(crate) enum ClaudeRecord {
    User(UserRecord),
    Assistant(AssistantRecord),
    Attachment(AttachmentRecord),
    System(SystemRecord),
    AgentName(AgentNameRecord),
    AiTitle(AiTitleRecord),
    AgentSetting(AgentSettingRecord),
    PermissionMode(PermissionModeRecord),
    ContinuedIn(ContinuedInRecord),
    CostState(CostStateRecord),
    QueueOperation(QueueOperationRecord),
    PrLink(PrLinkRecord),
    /// Kinds that are understood and intentionally produce no events
    /// (latest-wins state snapshots, header-only records, legacy kinds).
    #[serde(
        rename = "mode",
        alias = "atis-latch",
        alias = "last-prompt",
        alias = "bridge-session",
        alias = "frame-link",
        alias = "fork-context-ref",
        alias = "file-history-snapshot",
        alias = "progress",
        alias = "summary"
    )]
    Ignored(IgnoredRecord),
    #[serde(other)]
    Unknown,
}

impl ClaudeRecord {
    /// Envelope of a transcript record (`user`, `assistant`, `attachment`, `system`).
    pub(crate) fn envelope(&self) -> Option<&Envelope> {
        match self {
            Self::User(r) => Some(&r.env),
            Self::Assistant(r) => Some(&r.env),
            Self::Attachment(r) => Some(&r.env),
            Self::System(r) => Some(&r.env),
            _ => None,
        }
    }
}

/// Payload-less placeholder for ignored kinds (all fields are skipped).
#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct IgnoredRecord {}

/// Envelope fields shared by transcript records (`user`, `assistant`, `attachment`, `system`).
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Envelope {
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    /// Agent Teams: team of the teammate that wrote the record.
    #[serde(default)]
    pub team_name: Option<String>,
    /// Runtime session id of the process that wrote the record; differs from the
    /// transcript's `sessionId` after a resume / bg daemon respawn.
    #[serde(default, rename = "session_id")]
    pub runtime_session_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserRecord {
    #[serde(flatten)]
    pub env: Envelope,
    pub message: UserMessage,
    #[serde(default)]
    pub is_meta: bool,
    #[serde(default)]
    pub is_compact_summary: bool,
    /// Per-tool result details (dict / list / str depending on the tool).
    #[serde(default)]
    pub tool_use_result: Option<Value>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantRecord {
    #[serde(flatten)]
    pub env: Envelope,
    pub message: AssistantMessage,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttachmentRecord {
    #[serde(flatten)]
    pub env: Envelope,
    pub attachment: Attachment,
}

/// Inner attachment payload. Only the timeline-relevant types are modeled.
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Attachment {
    QueuedCommand(QueuedCommand),
    Model(ModelAttachment),
    TeamContext(TeamContext),
    PlanModeExit {
        #[serde(default, rename = "planFilePath")]
        plan_file_path: Option<String>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueuedCommand {
    #[serde(default)]
    pub prompt: Option<Value>,
    /// "prompt" (user input) | "task-notification"
    #[serde(default)]
    pub command_mode: Option<String>,
    #[serde(default)]
    pub origin: Option<QueuedOrigin>,
    #[serde(default)]
    pub usage: Option<QueuedUsage>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueuedOrigin {
    /// "human" | "task-notification" | "peer"
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub handback: bool,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueuedUsage {
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub tool_uses: Option<u64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct ModelAttachment {
    #[serde(default)]
    pub identity: Option<ModelIdentity>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelIdentity {
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub marketing_name: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TeamContext {
    #[serde(default)]
    pub team_name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemRecord {
    #[serde(flatten)]
    pub env: Envelope,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub level: Option<String>,
    /// turn_duration
    #[serde(default)]
    pub duration_ms: Option<u64>,
    /// turn_duration
    #[serde(default)]
    pub pending_background_agent_count: Option<u32>,
    /// compact_boundary
    #[serde(default)]
    pub compact_metadata: Option<CompactMetadata>,
    /// stop_hook_summary
    #[serde(default)]
    pub hook_count: Option<u32>,
    /// stop_hook_summary
    #[serde(default)]
    pub hook_infos: Option<Vec<HookInfo>>,
    /// local_command
    #[serde(default)]
    pub command_run: Option<CommandRun>,
    /// api_error
    #[serde(default)]
    pub retry_attempt: Option<u32>,
    /// api_error
    #[serde(default)]
    pub max_retries: Option<u32>,
    /// scheduled_task_fire
    #[serde(default)]
    pub prompt: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompactMetadata {
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub pre_tokens: Option<u64>,
    #[serde(default)]
    pub post_tokens: Option<u64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct HookInfo {
    #[serde(default)]
    pub command: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub(crate) struct CommandRun {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<String>,
}

// ---- latest-wins state records (no uuid / timestamp) ----

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentNameRecord {
    #[serde(default)]
    pub agent_name: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AiTitleRecord {
    #[serde(default)]
    pub ai_title: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentSettingRecord {
    #[serde(default)]
    pub agent_setting: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PermissionModeRecord {
    #[serde(default)]
    pub permission_mode: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContinuedInRecord {
    #[serde(default)]
    pub continued_in_session_id: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CostStateRecord {
    /// Model id -> per-model totals. Only the keys are used (`[1m]` markers).
    #[serde(default)]
    pub model_usage: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueueOperationRecord {
    #[serde(default)]
    pub operation: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrLinkRecord {
    #[serde(default)]
    pub pr_number: Option<u64>,
    #[serde(default)]
    pub pr_url: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> ClaudeRecord {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn ignored_kinds_accept_any_fields() {
        for kind in [
            "mode",
            "atis-latch",
            "last-prompt",
            "bridge-session",
            "frame-link",
            "fork-context-ref",
            "file-history-snapshot",
        ] {
            let rec = parse(&format!(
                r#"{{"type":"{kind}","x":1,"nested":{{"a":[1]}}}}"#
            ));
            assert!(matches!(rec, ClaudeRecord::Ignored(_)), "{kind}");
        }
    }

    #[test]
    fn unknown_kind_is_unknown() {
        assert!(matches!(
            parse(r#"{"type":"future-kind","x":1}"#),
            ClaudeRecord::Unknown
        ));
    }

    #[test]
    fn user_string_and_array_content() {
        let ClaudeRecord::User(u) = parse(r#"{"type":"user","message":{"content":"hi"}}"#) else {
            panic!()
        };
        assert_eq!(u.message.content.len(), 1);
        let ClaudeRecord::User(u) = parse(
            r#"{"type":"user","message":{"content":[{"type":"image","source":{}},{"type":"tool_result","tool_use_id":"t","content":[{"type":"text","text":"x"}]}]}}"#,
        ) else {
            panic!()
        };
        assert_eq!(u.message.content.len(), 2);
    }

    #[test]
    fn unknown_attachment_is_other() {
        let ClaudeRecord::Attachment(a) =
            parse(r#"{"type":"attachment","attachment":{"type":"skill_listing","names":[]}}"#)
        else {
            panic!()
        };
        assert!(matches!(a.attachment, Attachment::Other));
    }
}
