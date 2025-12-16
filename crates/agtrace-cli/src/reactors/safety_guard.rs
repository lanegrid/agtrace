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
                    // Path traversal check
                    if s.contains("..") {
                        return Some(format!("Path traversal detected: '{}'", s));
                    }

                    // Absolute path outside user directories
                    if s.starts_with('/') && !s.starts_with("/Users/") && !s.starts_with("/home/") {
                        return Some(format!("Absolute path outside user directory: '{}'", s));
                    }

                    // Root path access
                    if s == "/" || s.starts_with("/etc/") || s.starts_with("/sys/") {
                        return Some(format!("System directory access: '{}'", s));
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
