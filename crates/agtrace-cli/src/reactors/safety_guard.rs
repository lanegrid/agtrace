use crate::reactor::{Reaction, Reactor, ReactorContext};
use agtrace_types::EventPayload;
use anyhow::Result;

// NOTE: SafetyGuard Design Rationale
//
// Why detect dangerous operations?
// - Agents can make mistakes or be misguided by user prompts
// - Path traversal (../) can escape intended sandbox boundaries
// - System directory access (/etc/, /sys/, /) can cause irreversible damage
// - Absolute paths outside user directories may indicate unintended targets

/// SafetyGuard - detects potentially dangerous operations
pub struct SafetyGuard;

impl SafetyGuard {
    pub fn new() -> Self {
        Self
    }

    /// Check for dangerous patterns in tool arguments
    fn check_danger(&self, args: &serde_json::Value) -> Option<String> {
        if let Some(obj) = args.as_object() {
            for (_key, value) in obj.iter() {
                if let Some(s) = value.as_str() {
                    // Path traversal check: detect ".." as path component, not any occurrence
                    if s == ".." || s.starts_with("../") || s.contains("/../") || s.ends_with("/..")
                    {
                        return Some(format!("Path traversal detected: '{}'", s));
                    }

                    // Root/system path access (check first before general absolute path check)
                    if s == "/" || s.starts_with("/etc/") || s.starts_with("/sys/") {
                        return Some(format!("System directory access: '{}'", s));
                    }

                    // Absolute path outside user directories
                    if s.starts_with('/') && !s.starts_with("/Users/") && !s.starts_with("/home/") {
                        return Some(format!("Absolute path outside user directory: '{}'", s));
                    }
                }
            }
        }
        None
    }
}

impl Reactor for SafetyGuard {
    fn name(&self) -> &str {
        "SafetyGuard"
    }

    fn handle(&mut self, ctx: ReactorContext) -> Result<Reaction> {
        // Only check ToolCall events
        if let EventPayload::ToolCall(payload) = &ctx.event.payload {
            if let Some(danger_msg) = self.check_danger(&payload.arguments) {
                return Ok(Reaction::Warn(format!(
                    "Dangerous operation in {}: {}",
                    payload.name, danger_msg
                )));
            }
        }

        Ok(Reaction::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_types::{AgentEvent, EventPayload, StreamId, ToolCallPayload, UserPayload};
    use chrono::Utc;

    fn create_tool_call_event(args: serde_json::Value) -> AgentEvent {
        use std::str::FromStr;
        let id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap();
        let trace_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap();

        AgentEvent {
            id,
            trace_id,
            parent_id: None,
            timestamp: Utc::now(),
            stream_id: StreamId::Main,
            payload: EventPayload::ToolCall(ToolCallPayload {
                name: "Read".to_string(),
                arguments: args,
                provider_call_id: None,
            }),
            metadata: None,
        }
    }

    fn create_user_event() -> AgentEvent {
        use std::str::FromStr;
        let id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000003").unwrap();
        let trace_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000004").unwrap();

        AgentEvent {
            id,
            trace_id,
            parent_id: None,
            timestamp: Utc::now(),
            stream_id: StreamId::Main,
            payload: EventPayload::User(UserPayload {
                text: "test".to_string(),
            }),
            metadata: None,
        }
    }

    #[test]
    fn test_safe_path_allowed() {
        let mut guard = SafetyGuard::new();
        let event = create_tool_call_event(serde_json::json!({
            "path": "/Users/test/project/file.rs"
        }));
        let state = crate::reactor::SessionState::new("test".to_string(), None, Utc::now());
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = guard.handle(ctx).unwrap();
        assert!(matches!(result, Reaction::Continue));
    }

    #[test]
    fn test_path_traversal_detected() {
        let mut guard = SafetyGuard::new();
        let event = create_tool_call_event(serde_json::json!({
            "path": "../../../etc/passwd"
        }));
        let state = crate::reactor::SessionState::new("test".to_string(), None, Utc::now());
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = guard.handle(ctx).unwrap();
        match result {
            Reaction::Warn(reason) => {
                assert!(reason.contains("Path traversal"));
            }
            _ => panic!("Expected Warn reaction for path traversal"),
        }
    }

    #[test]
    fn test_system_directory_detected() {
        let mut guard = SafetyGuard::new();
        let event = create_tool_call_event(serde_json::json!({
            "path": "/etc/passwd"
        }));
        let state = crate::reactor::SessionState::new("test".to_string(), None, Utc::now());
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = guard.handle(ctx).unwrap();
        match result {
            Reaction::Warn(reason) => {
                assert!(reason.contains("System directory"));
            }
            _ => panic!("Expected Warn reaction for system directory"),
        }
    }

    #[test]
    fn test_root_path_detected() {
        let mut guard = SafetyGuard::new();
        let event = create_tool_call_event(serde_json::json!({
            "path": "/"
        }));
        let state = crate::reactor::SessionState::new("test".to_string(), None, Utc::now());
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = guard.handle(ctx).unwrap();
        match result {
            Reaction::Warn(reason) => {
                assert!(reason.contains("System directory"));
            }
            _ => panic!("Expected Warn reaction for root path"),
        }
    }

    #[test]
    fn test_absolute_path_outside_user_detected() {
        let mut guard = SafetyGuard::new();
        let event = create_tool_call_event(serde_json::json!({
            "path": "/opt/secret/file"
        }));
        let state = crate::reactor::SessionState::new("test".to_string(), None, Utc::now());
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = guard.handle(ctx).unwrap();
        match result {
            Reaction::Warn(reason) => {
                assert!(reason.contains("outside user directory"));
            }
            _ => panic!("Expected Warn reaction for path outside user directory"),
        }
    }

    #[test]
    fn test_non_tool_call_event_ignored() {
        let mut guard = SafetyGuard::new();
        let event = create_user_event();
        let state = crate::reactor::SessionState::new("test".to_string(), None, Utc::now());
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = guard.handle(ctx).unwrap();
        assert!(matches!(result, Reaction::Continue));
    }

    #[test]
    fn test_relative_path_allowed() {
        let mut guard = SafetyGuard::new();
        let event = create_tool_call_event(serde_json::json!({
            "path": "src/main.rs"
        }));
        let state = crate::reactor::SessionState::new("test".to_string(), None, Utc::now());
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = guard.handle(ctx).unwrap();
        assert!(matches!(result, Reaction::Continue));
    }

    #[test]
    fn test_dots_in_filename_allowed() {
        let mut guard = SafetyGuard::new();
        // Path with consecutive dots but not path traversal
        let event = create_tool_call_event(serde_json::json!({
            "path": "/Users/test/reactor...md"
        }));
        let state = crate::reactor::SessionState::new("test".to_string(), None, Utc::now());
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = guard.handle(ctx).unwrap();
        assert!(matches!(result, Reaction::Continue));
    }

    #[test]
    fn test_truncated_display_string_allowed() {
        let mut guard = SafetyGuard::new();
        // Truncated path display with "..." suffix should not trigger false positive
        let event = create_tool_call_event(serde_json::json!({
            "path": "/Users/zawakin/go/src/github.com/lanegrid/agtrace/docs/react..."
        }));
        let state = crate::reactor::SessionState::new("test".to_string(), None, Utc::now());
        let ctx = ReactorContext {
            event: &event,
            state: &state,
        };

        let result = guard.handle(ctx).unwrap();
        assert!(matches!(result, Reaction::Continue));
    }
}
