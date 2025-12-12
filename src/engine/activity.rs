use crate::model::{AgentEventV1, EventType, Role, ToolName, ToolStatus};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Activity {
    Message {
        role: Role,
        text: String,
        timestamp: String,
        duration_ms: Option<u64>,
        stats: ActivityStats,
    },
    Execution {
        timestamp: String,
        duration_ms: u64,
        status: ActivityStatus,
        tools: Vec<ToolSummary>,
        stats: ActivityStats,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSummary {
    pub name: String,
    pub input_summary: String,
    pub count: usize,
    pub is_error: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityStats {
    pub total_tokens: Option<u64>,
    pub event_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityStatus {
    Success,
    Failure,
    LongRunning,
}

impl Activity {
    pub fn timestamp(&self) -> &str {
        match self {
            Activity::Message { timestamp, .. } => timestamp,
            Activity::Execution { timestamp, .. } => timestamp,
        }
    }

    pub fn duration_ms(&self) -> Option<u64> {
        match self {
            Activity::Message { duration_ms, .. } => *duration_ms,
            Activity::Execution { duration_ms, .. } => Some(*duration_ms),
        }
    }

    pub fn stats(&self) -> &ActivityStats {
        match self {
            Activity::Message { stats, .. } => stats,
            Activity::Execution { stats, .. } => stats,
        }
    }
}

pub fn interpret_events(events: &[AgentEventV1]) -> Vec<Activity> {
    let mut activities = Vec::new();
    let mut buffer = ExecutionBuffer::new();

    for event in events {
        match event.event_type {
            EventType::UserMessage | EventType::AssistantMessage => {
                if !buffer.is_empty() {
                    if let Some(activity) = buffer.to_activity() {
                        activities.push(activity);
                    }
                    buffer = ExecutionBuffer::new();
                }

                let role =
                    event
                        .role
                        .unwrap_or(if matches!(event.event_type, EventType::UserMessage) {
                            Role::User
                        } else {
                            Role::Assistant
                        });

                let text = event.text.clone().unwrap_or_default();
                let text = normalize_text(&text);

                activities.push(Activity::Message {
                    role,
                    text,
                    timestamp: event.ts.clone(),
                    duration_ms: None,
                    stats: ActivityStats {
                        total_tokens: event.tokens_total.or_else(|| {
                            event
                                .tokens_input
                                .and_then(|input| event.tokens_output.map(|output| input + output))
                        }),
                        event_count: 1,
                    },
                });
            }

            EventType::Reasoning => {
                buffer.mark_start(&event.ts);
                buffer.event_count += 1;
            }

            EventType::ToolCall => {
                buffer.push_tool(event);
                buffer.event_count += 1;
            }

            EventType::ToolResult => {
                buffer.update_status(event);
                buffer.mark_end(&event.ts);
                buffer.event_count += 1;
            }

            _ => {}
        }
    }

    if !buffer.is_empty() {
        if let Some(activity) = buffer.to_activity() {
            activities.push(activity);
        }
    }

    activities
}

struct ExecutionBuffer {
    tools: Vec<ToolInfo>,
    start_ts: Option<DateTime<chrono::FixedOffset>>,
    end_ts: Option<DateTime<chrono::FixedOffset>>,
    event_count: usize,
}

struct ToolInfo {
    name: String,
    raw_target: Option<String>,
    display_target: Option<String>,
    call_id: Option<String>,
    status: Option<ToolStatus>,
}

impl ExecutionBuffer {
    fn new() -> Self {
        Self {
            tools: Vec::new(),
            start_ts: None,
            end_ts: None,
            event_count: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    fn mark_start(&mut self, ts: &str) {
        if let Ok(parsed_ts) = DateTime::parse_from_rfc3339(ts) {
            if self.start_ts.is_none() {
                self.start_ts = Some(parsed_ts);
            }
        }
    }

    fn push_tool(&mut self, event: &AgentEventV1) {
        if let Some(name) = &event.tool_name {
            let (raw_target, display_target) =
                extract_target_summary(name, &event.file_path, &event.text);

            self.tools.push(ToolInfo {
                name: name.clone(),
                raw_target,
                display_target,
                call_id: event.tool_call_id.clone(),
                status: event.tool_status,
            });
        }
        if let Ok(parsed_ts) = DateTime::parse_from_rfc3339(&event.ts) {
            if self.start_ts.is_none() {
                self.start_ts = Some(parsed_ts);
            }
            self.end_ts = Some(parsed_ts);
        }
    }

    fn update_status(&mut self, event: &AgentEventV1) {
        if let Some(id) = &event.tool_call_id {
            for tool in self.tools.iter_mut().rev() {
                if tool.call_id.as_ref() == Some(id) {
                    tool.status = event.tool_status;
                    break;
                }
            }
        }
    }

    fn mark_end(&mut self, ts: &str) {
        if let Ok(parsed_ts) = DateTime::parse_from_rfc3339(ts) {
            self.end_ts = Some(parsed_ts);
        }
    }

    fn to_activity(&self) -> Option<Activity> {
        if self.is_empty() {
            return None;
        }

        let duration_ms = if let (Some(start), Some(end)) = (self.start_ts, self.end_ts) {
            end.signed_duration_since(start).num_milliseconds().max(0) as u64
        } else {
            0
        };

        let tools = self.aggregate_tools();
        let status = self.calculate_status(duration_ms);

        Some(Activity::Execution {
            timestamp: self.start_ts.map(|ts| ts.to_rfc3339()).unwrap_or_default(),
            duration_ms,
            status,
            tools,
            stats: ActivityStats {
                total_tokens: None,
                event_count: self.event_count,
            },
        })
    }

    fn aggregate_tools(&self) -> Vec<ToolSummary> {
        let mut summaries = Vec::new();
        let mut i = 0;

        while i < self.tools.len() {
            let current_tool = &self.tools[i].name;
            let current_raw = &self.tools[i].raw_target;
            let current_status = &self.tools[i].status;
            let mut count = 1;

            while i + count < self.tools.len()
                && &self.tools[i + count].name == current_tool
                && &self.tools[i + count].raw_target == current_raw
                && &self.tools[i + count].status == current_status
            {
                count += 1;
            }

            let is_error = matches!(current_status, Some(ToolStatus::Error));
            let input_summary = self.tools[i].display_target.clone().unwrap_or_default();

            summaries.push(ToolSummary {
                name: current_tool.clone(),
                input_summary,
                count,
                is_error,
            });

            i += count;
        }

        summaries
    }

    fn calculate_status(&self, duration_ms: u64) -> ActivityStatus {
        let has_error = self
            .tools
            .iter()
            .any(|t| matches!(t.status, Some(ToolStatus::Error)));

        if has_error {
            ActivityStatus::Failure
        } else if duration_ms > 30000 {
            ActivityStatus::LongRunning
        } else {
            ActivityStatus::Success
        }
    }
}

fn normalize_text(text: &str) -> String {
    let text_normalized = text.replace('\n', " ");
    let clean_text: String = text_normalized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let limit = 100;
    let chars: Vec<char> = clean_text.chars().collect();
    if chars.len() > limit {
        let preview: String = chars.iter().take(limit).collect();
        format!("{}...", preview)
    } else {
        clean_text
    }
}

fn extract_target_summary(
    tool_name: &str,
    file_path: &Option<String>,
    text: &Option<String>,
) -> (Option<String>, Option<String>) {
    let tool = ToolName::from_str(tool_name).unwrap_or(ToolName::Other(tool_name.to_string()));

    match tool {
        ToolName::Read | ToolName::Edit | ToolName::Write => {
            let filename = file_path.as_ref().and_then(|p| {
                std::path::Path::new(p)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            });
            (filename.clone(), filename)
        }
        ToolName::Bash => text
            .as_ref()
            .and_then(|t| {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(t) {
                    json.get("command").and_then(|v| v.as_str()).map(|cmd| {
                        let cmd = cmd.trim();
                        let sanitized = sanitize_bash_command(cmd);
                        let display = truncate_bash_display(&sanitized, 80);
                        (sanitized, display)
                    })
                } else {
                    let cmd = t.trim();
                    let sanitized = sanitize_bash_command(cmd);
                    let display = truncate_bash_display(&sanitized, 80);
                    Some((sanitized, display))
                }
            })
            .map(|(raw, display)| (Some(raw), Some(display)))
            .unwrap_or((None, None)),
        ToolName::Glob | ToolName::Grep => text
            .as_ref()
            .and_then(|t| {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(t) {
                    json.get("pattern").and_then(|v| v.as_str()).map(|p| {
                        let raw = format!("\"{}\"", p);
                        let display = if p.len() > 30 {
                            format!("\"{}...\"", &p.chars().take(27).collect::<String>())
                        } else {
                            format!("\"{}\"", p)
                        };
                        (Some(raw), Some(display))
                    })
                } else {
                    None
                }
            })
            .unwrap_or((None, None)),
        ToolName::Other(_) => (None, None),
    }
}

fn sanitize_bash_command(cmd: &str) -> String {
    if cmd.contains("git commit") && cmd.contains("<<") {
        if let Some(pos) = cmd.find("git commit") {
            let prefix = &cmd[pos..];
            if prefix.contains("-m") {
                return "git commit -m \"...\"".to_string();
            }
        }
    }
    cmd.to_string()
}

fn truncate_bash_display(cmd: &str, limit: usize) -> String {
    if cmd.chars().count() <= limit {
        return cmd.to_string();
    }

    let tokens = tokenize_bash_command(cmd);
    if tokens.is_empty() {
        return truncate_simple(cmd, limit);
    }

    let first_token = &tokens[0];
    let compressed_first = if first_token.contains('/') {
        compress_path_for_display(first_token)
    } else {
        first_token.to_string()
    };

    let mut result = compressed_first;
    let mut added_all = true;

    for token in tokens.iter().skip(1) {
        let next_len = result.chars().count() + 1 + token.chars().count();

        if next_len > limit {
            if tokens.len() > 1 && result.chars().count() + 4 <= limit {
                result.push_str(" ...");
            }
            added_all = false;
            break;
        }

        result.push(' ');
        result.push_str(token);
    }

    if !added_all && result.chars().count() > limit {
        truncate_simple(&result, limit)
    } else {
        result
    }
}

fn tokenize_bash_command(cmd: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';
    let chars: Vec<char> = cmd.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if in_quotes {
            current.push(ch);
            if ch == quote_char && (i == 0 || chars[i - 1] != '\\') {
                in_quotes = false;
            }
        } else if ch == '"' || ch == '\'' {
            in_quotes = true;
            quote_char = ch;
            current.push(ch);
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }

        i += 1;
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn compress_path_for_display(path: &str) -> String {
    if path.starts_with("./target/release/") || path.starts_with("./target/debug/") {
        if let Some(filename) = path.split('/').next_back() {
            return format!(".../{}", filename);
        }
    }

    if path.matches('/').count() >= 2 {
        if let Some(filename) = path.split('/').next_back() {
            return format!(".../{}", filename);
        }
    }

    path.to_string()
}

fn truncate_simple(s: &str, limit: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= limit {
        s.to_string()
    } else {
        format!("{}...", chars[..limit - 3].iter().collect::<String>())
    }
}
