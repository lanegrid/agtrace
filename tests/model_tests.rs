use agtrace::model::*;
use serde_json;

#[test]
fn test_source_serialization() {
    let sources = vec![
        (Source::ClaudeCode, "claude_code"),
        (Source::Codex, "codex"),
        (Source::Gemini, "gemini"),
    ];

    for (source, expected) in sources {
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));

        let deserialized: Source = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, source);
    }
}

#[test]
fn test_event_type_serialization() {
    let types = vec![
        (EventType::UserMessage, "user_message"),
        (EventType::AssistantMessage, "assistant_message"),
        (EventType::SystemMessage, "system_message"),
        (EventType::Reasoning, "reasoning"),
        (EventType::ToolCall, "tool_call"),
        (EventType::ToolResult, "tool_result"),
        (EventType::FileSnapshot, "file_snapshot"),
        (EventType::SessionSummary, "session_summary"),
        (EventType::Meta, "meta"),
        (EventType::Log, "log"),
    ];

    for (event_type, expected) in types {
        let json = serde_json::to_string(&event_type).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));
    }
}

#[test]
fn test_role_serialization() {
    let roles = vec![
        (Role::User, "user"),
        (Role::Assistant, "assistant"),
        (Role::System, "system"),
        (Role::Tool, "tool"),
        (Role::Cli, "cli"),
        (Role::Other, "other"),
    ];

    for (role, expected) in roles {
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));
    }
}

#[test]
fn test_channel_serialization() {
    let channels = vec![
        (Channel::Chat, "chat"),
        (Channel::Editor, "editor"),
        (Channel::Terminal, "terminal"),
        (Channel::Filesystem, "filesystem"),
        (Channel::System, "system"),
        (Channel::Other, "other"),
    ];

    for (channel, expected) in channels {
        let json = serde_json::to_string(&channel).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));
    }
}

#[test]
fn test_tool_status_serialization() {
    let statuses = vec![
        (ToolStatus::Success, "success"),
        (ToolStatus::Error, "error"),
        (ToolStatus::InProgress, "in_progress"),
        (ToolStatus::Unknown, "unknown"),
    ];

    for (status, expected) in statuses {
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));
    }
}

#[test]
fn test_agent_event_v1_new() {
    let event = AgentEventV1::new(
        Source::ClaudeCode,
        "abc123".to_string(),
        "2025-11-26T12:51:28.093Z".to_string(),
        EventType::UserMessage,
    );

    assert_eq!(event.schema_version, AgentEventV1::SCHEMA_VERSION);
    assert_eq!(event.source, Source::ClaudeCode);
    assert_eq!(event.project_hash, "abc123");
    assert_eq!(event.ts, "2025-11-26T12:51:28.093Z");
    assert_eq!(event.event_type, EventType::UserMessage);
    assert_eq!(event.project_root, None);
    assert_eq!(event.session_id, None);
}

#[test]
fn test_agent_event_v1_serialization() {
    let mut event = AgentEventV1::new(
        Source::Codex,
        "def456".to_string(),
        "2025-11-03T01:49:22.517Z".to_string(),
        EventType::ToolCall,
    );

    event.session_id = Some("session-1".to_string());
    event.event_id = Some("evt-001".to_string());
    event.parent_event_id = Some("evt-000".to_string());
    event.role = Some(Role::Assistant);
    event.channel = Some(Channel::Terminal);
    event.text = Some("ls -la".to_string());
    event.tool_name = Some("Bash".to_string());
    event.tool_call_id = Some("call-001".to_string());

    // Serialize
    let json = serde_json::to_string(&event).unwrap();

    // Deserialize
    let deserialized: AgentEventV1 = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.schema_version, AgentEventV1::SCHEMA_VERSION);
    assert_eq!(deserialized.source, Source::Codex);
    assert_eq!(deserialized.session_id, Some("session-1".to_string()));
    assert_eq!(deserialized.event_id, Some("evt-001".to_string()));
    assert_eq!(deserialized.tool_name, Some("Bash".to_string()));
}

#[test]
fn test_agent_event_v1_with_tokens() {
    let mut event = AgentEventV1::new(
        Source::Gemini,
        "ghi789".to_string(),
        "2025-12-07T17:17:16.876Z".to_string(),
        EventType::AssistantMessage,
    );

    event.model = Some("gemini-pro".to_string());
    event.tokens_input = Some(1234);
    event.tokens_output = Some(567);
    event.tokens_total = Some(1801);

    let json = serde_json::to_string(&event).unwrap();
    let deserialized: AgentEventV1 = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.model, Some("gemini-pro".to_string()));
    assert_eq!(deserialized.tokens_input, Some(1234));
    assert_eq!(deserialized.tokens_output, Some(567));
    assert_eq!(deserialized.tokens_total, Some(1801));
}
