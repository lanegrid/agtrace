use agtrace_engine::{AgentSession, AgentTurn};
use agtrace_types::v2::{AgentEvent, EventPayload};
use agtrace_types::ToolStatus;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct SessionDisplay {
    pub session_id: String,
    pub model: Option<String>,
    pub start_time: DateTime<Utc>,
    pub turns: Vec<TurnDisplay>,
    pub token_summary: TokenSummaryDisplay,
}

#[derive(Debug, Clone)]
pub struct TurnDisplay {
    pub turn_number: usize,
    pub timestamp: DateTime<Utc>,
    pub user_text: String,
    pub steps: Vec<StepDisplay>,
}

#[derive(Debug, Clone)]
pub struct StepDisplay {
    pub timestamp: DateTime<Utc>,
    pub content: StepContent,
}

#[derive(Debug, Clone)]
pub enum StepContent {
    Reasoning { text: String },
    Tools { executions: Vec<ToolDisplay> },
    Message { text: String },
}

#[derive(Debug, Clone)]
pub struct ToolDisplay {
    pub name: String,
    pub arguments_summary: String,
    pub status: ToolStatus,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct TokenSummaryDisplay {
    pub input: i32,
    pub output: i32,
    pub cache_creation: i32,
    pub cache_read: i32,
    pub total: i32,
    pub limit: Option<u64>,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DisplayOptions {
    pub enable_color: bool,
    pub relative_time: bool,
    pub truncate_text: Option<usize>,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            enable_color: true,
            relative_time: true,
            truncate_text: None,
        }
    }
}

impl SessionDisplay {
    pub fn from_agent_session(session: &AgentSession) -> Self {
        let token_summary = calculate_token_summary_from_session(session);

        Self {
            session_id: session.session_id.to_string(),
            model: None,
            start_time: session.start_time,
            turns: session
                .turns
                .iter()
                .enumerate()
                .map(|(i, turn)| TurnDisplay::from_agent_turn(turn, i))
                .collect(),
            token_summary,
        }
    }

    pub fn from_events_snapshot(
        session_id: String,
        model: Option<String>,
        start_time: DateTime<Utc>,
        events: &[AgentEvent],
        limit: Option<u64>,
    ) -> Self {
        let turns = build_turns_from_events(events);
        let token_summary = calculate_token_summary_from_events(events, limit);

        Self {
            session_id,
            model,
            start_time,
            turns,
            token_summary,
        }
    }
}

impl TurnDisplay {
    fn from_agent_turn(turn: &AgentTurn, turn_number: usize) -> Self {
        let mut steps = Vec::new();

        for step in &turn.steps {
            if let Some(reasoning) = &step.reasoning {
                if !reasoning.content.text.trim().is_empty() {
                    steps.push(StepDisplay {
                        timestamp: step.timestamp,
                        content: StepContent::Reasoning {
                            text: reasoning.content.text.clone(),
                        },
                    });
                }
            }

            if !step.tools.is_empty() {
                let executions = step
                    .tools
                    .iter()
                    .map(|tool_exec| {
                        let status = if tool_exec.is_error {
                            ToolStatus::Error
                        } else if tool_exec.result.is_some() {
                            ToolStatus::Success
                        } else {
                            ToolStatus::Unknown
                        };

                        let arguments_summary = extract_input_summary(&tool_exec.call.content);

                        ToolDisplay {
                            name: tool_exec.call.content.name.clone(),
                            arguments_summary,
                            status,
                            duration_ms: tool_exec.duration_ms.map(|d| d as u64),
                        }
                    })
                    .collect();

                steps.push(StepDisplay {
                    timestamp: step.timestamp,
                    content: StepContent::Tools { executions },
                });
            }

            if let Some(msg) = &step.message {
                if !msg.content.text.trim().is_empty() {
                    steps.push(StepDisplay {
                        timestamp: step.timestamp,
                        content: StepContent::Message {
                            text: msg.content.text.clone(),
                        },
                    });
                }
            }
        }

        Self {
            turn_number,
            timestamp: turn.timestamp,
            user_text: turn.user.content.text.clone(),
            steps,
        }
    }
}

fn extract_input_summary(payload: &agtrace_types::v2::ToolCallPayload) -> String {
    if let Some(file_path) = payload.arguments.get("file_path").and_then(|v| v.as_str()) {
        if let Some(filename) = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
        {
            return filename.to_string();
        }
    }

    if let Some(cmd) = payload.arguments.get("command").and_then(|v| v.as_str()) {
        return truncate_string(cmd, 50);
    }

    if let Some(pattern) = payload.arguments.get("pattern").and_then(|v| v.as_str()) {
        return format!("\"{}\"", truncate_string(pattern, 30));
    }

    String::new()
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let chars: Vec<char> = s.chars().take(max_len - 3).collect();
        format!("{}...", chars.iter().collect::<String>())
    }
}

fn calculate_token_summary_from_session(session: &AgentSession) -> TokenSummaryDisplay {
    let mut total_input = 0i32;
    let mut total_output = 0i32;
    let mut total_cache_creation = 0i32;
    let mut total_cache_read = 0i32;

    for turn in &session.turns {
        for step in &turn.steps {
            if let Some(usage) = &step.usage {
                total_input += usage.input_tokens;
                total_output += usage.output_tokens;

                if let Some(details) = &usage.details {
                    if let Some(cache_creation) = details.cache_creation_input_tokens {
                        total_cache_creation += cache_creation;
                    }
                    if let Some(cache_read) = details.cache_read_input_tokens {
                        total_cache_read += cache_read;
                    }
                }
            }
        }
    }

    let total = total_input + total_output;

    TokenSummaryDisplay {
        input: total_input,
        output: total_output,
        cache_creation: total_cache_creation,
        cache_read: total_cache_read,
        total,
        limit: None,
        model: None,
    }
}

fn calculate_token_summary_from_events(
    events: &[AgentEvent],
    limit: Option<u64>,
) -> TokenSummaryDisplay {
    let mut total_input = 0i32;
    let mut total_output = 0i32;
    let mut total_cache_creation = 0i32;
    let mut total_cache_read = 0i32;

    for event in events {
        if let EventPayload::TokenUsage(usage) = &event.payload {
            total_input += usage.input_tokens;
            total_output += usage.output_tokens;

            if let Some(details) = &usage.details {
                if let Some(cache_creation) = details.cache_creation_input_tokens {
                    total_cache_creation += cache_creation;
                }
                if let Some(cache_read) = details.cache_read_input_tokens {
                    total_cache_read += cache_read;
                }
            }
        }
    }

    let total = total_input + total_output;

    TokenSummaryDisplay {
        input: total_input,
        output: total_output,
        cache_creation: total_cache_creation,
        cache_read: total_cache_read,
        total,
        limit,
        model: None,
    }
}

fn build_turns_from_events(events: &[AgentEvent]) -> Vec<TurnDisplay> {
    let mut turns = Vec::new();
    let mut current_turn: Option<TurnDisplay> = None;
    let mut current_turn_number = 0;

    for event in events {
        match &event.payload {
            EventPayload::User(payload) => {
                if let Some(turn) = current_turn.take() {
                    turns.push(turn);
                }

                current_turn = Some(TurnDisplay {
                    turn_number: current_turn_number,
                    timestamp: event.timestamp,
                    user_text: payload.text.clone(),
                    steps: Vec::new(),
                });
                current_turn_number += 1;
            }
            EventPayload::Reasoning(payload) => {
                if let Some(ref mut turn) = current_turn {
                    if !payload.text.trim().is_empty() {
                        turn.steps.push(StepDisplay {
                            timestamp: event.timestamp,
                            content: StepContent::Reasoning {
                                text: payload.text.clone(),
                            },
                        });
                    }
                }
            }
            EventPayload::ToolCall(payload) => {
                if let Some(ref mut turn) = current_turn {
                    let arguments_summary =
                        extract_input_summary_from_tool_call(&payload.name, &payload.arguments);

                    turn.steps.push(StepDisplay {
                        timestamp: event.timestamp,
                        content: StepContent::Tools {
                            executions: vec![ToolDisplay {
                                name: payload.name.clone(),
                                arguments_summary,
                                status: ToolStatus::InProgress,
                                duration_ms: None,
                            }],
                        },
                    });
                }
            }
            EventPayload::ToolResult(payload) => {
                if let Some(ref mut turn) = current_turn {
                    if let Some(last_step) = turn.steps.last_mut() {
                        if let StepContent::Tools { executions } = &mut last_step.content {
                            if let Some(last_tool) = executions.last_mut() {
                                last_tool.status = if payload.is_error {
                                    ToolStatus::Error
                                } else {
                                    ToolStatus::Success
                                };
                            }
                        }
                    }
                }
            }
            EventPayload::Message(payload) => {
                if let Some(ref mut turn) = current_turn {
                    if !payload.text.trim().is_empty() {
                        turn.steps.push(StepDisplay {
                            timestamp: event.timestamp,
                            content: StepContent::Message {
                                text: payload.text.clone(),
                            },
                        });
                    }
                }
            }
            EventPayload::TokenUsage(_) | EventPayload::Notification(_) => {}
        }
    }

    if let Some(turn) = current_turn {
        turns.push(turn);
    }

    turns
}

fn extract_input_summary_from_tool_call(_name: &str, args: &serde_json::Value) -> String {
    if let Some(obj) = args.as_object() {
        if let Some(path) = obj.get("path").or_else(|| obj.get("file_path")) {
            if let Some(path_str) = path.as_str() {
                if let Some(filename) = std::path::Path::new(path_str)
                    .file_name()
                    .and_then(|n| n.to_str())
                {
                    return filename.to_string();
                }
            }
        }
        if let Some(command) = obj.get("command") {
            if let Some(cmd_str) = command.as_str() {
                return truncate_string(cmd_str, 50);
            }
        }
        if let Some(pattern) = obj.get("pattern") {
            if let Some(pat_str) = pattern.as_str() {
                return format!("\"{}\"", truncate_string(pat_str, 30));
            }
        }
    }

    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
        assert_eq!(truncate_string("日本語テスト", 5), "日本...");
    }

    #[test]
    fn test_token_summary_display() {
        let summary = TokenSummaryDisplay {
            input: 1000,
            output: 500,
            cache_creation: 200,
            cache_read: 800,
            total: 1500,
            limit: Some(200_000),
            model: Some("claude-3-5-sonnet-20241022".to_string()),
        };

        assert_eq!(summary.input, 1000);
        assert_eq!(summary.output, 500);
        assert_eq!(summary.total, 1500);
    }

    #[test]
    fn test_display_options_default() {
        let opts = DisplayOptions::default();
        assert!(opts.enable_color);
        assert!(opts.relative_time);
        assert!(opts.truncate_text.is_none());
    }
}
