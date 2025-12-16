use crate::reactor::{Reaction, Reactor, ReactorContext, Severity};
use agtrace_types::v2::EventPayload;
use anyhow::Result;

/// SafetyGuard - detects potentially dangerous operations
pub struct SafetyGuard {
    /// Whether to emit Kill severity for dangerous operations
    /// (false for v0.1.0 - monitoring only, true for v0.2.0 - intervention)
    kill_on_danger: bool,
}

impl SafetyGuard {
    pub fn new() -> Self {
        Self {
            kill_on_danger: false, // v0.1.0: monitoring only
        }
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
                let severity = if self.kill_on_danger {
                    Severity::Kill
                } else {
                    Severity::Notification
                };

                return Ok(Reaction::Intervene {
                    reason: format!("Dangerous operation in {}: {}", payload.name, danger_msg),
                    severity,
                });
            }
        }

        Ok(Reaction::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_types::v2::{AgentEvent, EventPayload, ToolCallPayload, UserPayload};
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
            Reaction::Intervene { reason, severity } => {
                assert!(reason.contains("Path traversal"));
                assert_eq!(severity, Severity::Notification);
            }
            _ => panic!("Expected Intervene reaction for path traversal"),
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
            Reaction::Intervene { reason, .. } => {
                assert!(reason.contains("System directory"));
            }
            _ => panic!("Expected Intervene reaction for system directory"),
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
            Reaction::Intervene { reason, .. } => {
                assert!(reason.contains("System directory"));
            }
            _ => panic!("Expected Intervene reaction for root path"),
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
            Reaction::Intervene { reason, .. } => {
                assert!(reason.contains("outside user directory"));
            }
            _ => panic!("Expected Intervene reaction for path outside user directory"),
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
