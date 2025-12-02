use crate::error::{Error, Result};
use crate::model::{Agent, Event, Execution, ExecutionMetrics, ToolStatus};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

mod raw;

pub use raw::CodexEvent;

/// Parse Codex sessions from the default directory (~/.codex/sessions)
pub fn parse_default_dir() -> Result<Vec<Execution>> {
    let home = home::home_dir()
        .ok_or_else(|| Error::Parse("Could not find home directory".to_string()))?;
    let codex_dir = home.join(".codex").join("sessions");
    parse_dir(&codex_dir)
}

/// Parse Codex sessions from a custom directory
/// Expects directory structure: sessions_dir/YYYY/MM/DD/*.jsonl
pub fn parse_dir(path: &Path) -> Result<Vec<Execution>> {
    if !path.exists() {
        return Err(Error::AgentDataNotFound(path.to_path_buf()));
    }

    let mut executions = Vec::new();

    // Scan YYYY directories (year)
    for year_entry in std::fs::read_dir(path)? {
        let year_entry = year_entry?;
        let year_path = year_entry.path();

        if !year_path.is_dir() {
            continue;
        }

        // Validate YYYY format (4 digits)
        let _year_name = match year_path.file_name().and_then(|s| s.to_str()) {
            Some(name) if name.len() == 4 && name.chars().all(|c| c.is_ascii_digit()) => name,
            _ => continue,
        };

        // Scan MM directories (month)
        for month_entry in std::fs::read_dir(&year_path)? {
            let month_entry = month_entry?;
            let month_path = month_entry.path();

            if !month_path.is_dir() {
                continue;
            }

            // Validate MM format (2 digits)
            let _month_name = match month_path.file_name().and_then(|s| s.to_str()) {
                Some(name) if name.len() == 2 && name.chars().all(|c| c.is_ascii_digit()) => name,
                _ => continue,
            };

            // Scan DD directories (day)
            for day_entry in std::fs::read_dir(&month_path)? {
                let day_entry = day_entry?;
                let day_path = day_entry.path();

                if !day_path.is_dir() {
                    continue;
                }

                // Validate DD format (2 digits)
                let _day_name = match day_path.file_name().and_then(|s| s.to_str()) {
                    Some(name) if name.len() == 2 && name.chars().all(|c| c.is_ascii_digit()) => {
                        name
                    }
                    _ => continue,
                };

                // Scan session files in YYYY/MM/DD directory
                for file_entry in std::fs::read_dir(&day_path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if !file_path.is_file() {
                        continue;
                    }

                    let filename = match file_path.file_name().and_then(|s| s.to_str()) {
                        Some(name) => name,
                        None => continue,
                    };

                    if filename.starts_with("rollout-") && filename.ends_with(".jsonl") {
                        match parse_session_file(&file_path) {
                            Ok(exec) => executions.push(exec),
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to parse {}: {}",
                                    file_path.display(),
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(executions)
}

/// Parse a single Codex session file (.jsonl)
fn parse_session_file(path: &Path) -> Result<Execution> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut session_id: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut git_branch: Option<String> = None;
    let mut started_at: Option<DateTime<Utc>> = None;
    let mut events = Vec::new();
    let mut instructions: Option<String> = None;
    let mut model: Option<String> = None;
    let mut last_timestamp: Option<DateTime<Utc>> = None;

    // Track function calls to match with outputs
    let mut pending_call_id: Option<String> = None;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let event: CodexEvent = serde_json::from_str(&line)
            .map_err(|e| Error::Parse(format!("Invalid JSONL line: {}", e)))?;

        let timestamp = parse_timestamp(&event.timestamp)?;
        if started_at.is_none() {
            started_at = Some(timestamp);
        }
        last_timestamp = Some(timestamp);

        match event.event_type.as_str() {
            "session_meta" => {
                // Extract session metadata
                if let Some(id) = event.payload.get("id").and_then(|v| v.as_str()) {
                    session_id = Some(id.to_string());
                }
                if let Some(cwd_val) = event.payload.get("cwd").and_then(|v| v.as_str()) {
                    cwd = Some(cwd_val.to_string());
                }
                if let Some(git) = event.payload.get("git") {
                    if let Some(branch) = git.get("branch").and_then(|v| v.as_str()) {
                        git_branch = Some(branch.to_string());
                    }
                }
                if let Some(instr) = event.payload.get("instructions").and_then(|v| v.as_str()) {
                    if !instr.is_empty() {
                        instructions = Some(instr.to_string());
                    }
                }
            }
            "turn_context" => {
                // Extract context information
                if cwd.is_none() {
                    if let Some(cwd_val) = event.payload.get("cwd").and_then(|v| v.as_str()) {
                        cwd = Some(cwd_val.to_string());
                    }
                }
                if model.is_none() {
                    if let Some(model_val) = event.payload.get("model").and_then(|v| v.as_str()) {
                        model = Some(model_val.to_string());
                    }
                }
            }
            "response_item" => {
                // Parse response items (messages, reasoning, function calls, etc.)
                if let Some(item_type) = event.payload.get("type").and_then(|v| v.as_str()) {
                    match item_type {
                        "message" => {
                            if let Some(role) = event.payload.get("role").and_then(|v| v.as_str()) {
                                if let Some(content) = event.payload.get("content") {
                                    let text = extract_text_from_content(content);

                                    if role == "user" {
                                        events.push(Event::UserMessage {
                                            content: text,
                                            timestamp,
                                        });
                                    } else if role == "assistant" {
                                        events.push(Event::AssistantMessage {
                                            content: text,
                                            timestamp,
                                        });
                                    }
                                }
                            }
                        }
                        "reasoning" => {
                            let content = if let Some(content_val) =
                                event.payload.get("content").and_then(|v| v.as_str())
                            {
                                content_val.to_string()
                            } else if let Some(summary) = event.payload.get("summary") {
                                extract_summary_text(summary)
                            } else {
                                "[Reasoning]".to_string()
                            };

                            events.push(Event::Thinking {
                                content,
                                duration_ms: None,
                                timestamp,
                            });
                        }
                        "function_call" => {
                            if let Some(name) = event.payload.get("name").and_then(|v| v.as_str()) {
                                let arguments = event
                                    .payload
                                    .get("arguments")
                                    .unwrap_or(&serde_json::Value::Null);

                                // Parse arguments if it's a JSON string
                                let parsed_input = if let Some(arg_str) = arguments.as_str() {
                                    serde_json::from_str(arg_str)
                                        .unwrap_or_else(|_| arguments.clone())
                                } else {
                                    arguments.clone()
                                };

                                let call_id = event
                                    .payload
                                    .get("call_id")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                pending_call_id = call_id.clone();

                                events.push(Event::ToolCall {
                                    name: name.to_string(),
                                    input: parsed_input,
                                    call_id,
                                    timestamp,
                                });
                            }
                        }
                        "function_call_output" => {
                            let output_str = event
                                .payload
                                .get("output")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            // Parse output if it's a JSON string
                            let (parsed_output, exit_code, raw_metadata) = if let Ok(output_json) =
                                serde_json::from_str::<serde_json::Value>(&output_str)
                            {
                                let output = output_json
                                    .get("output")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(&output_str)
                                    .to_string();

                                let exit_code = output_json
                                    .get("metadata")
                                    .and_then(|m| m.get("exit_code"))
                                    .and_then(|c| c.as_i64())
                                    .map(|c| c as i32);

                                let raw_metadata = output_json
                                    .get("metadata")
                                    .cloned()
                                    .map(|meta| json!({"provider": "codex", "metadata": meta}));

                                (output, exit_code, raw_metadata)
                            } else {
                                (output_str, None, None)
                            };

                            let call_id = event
                                .payload
                                .get("call_id")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .or_else(|| pending_call_id.take());

                            events.push(Event::ToolResult {
                                call_id,
                                output: parsed_output,
                                exit_code,
                                is_error: None,
                                status: derive_status(exit_code, None),
                                raw_metadata,
                                duration_ms: None,
                                timestamp,
                            });
                        }
                        _ => {
                            // Ignore other types
                        }
                    }
                }
            }
            _ => {
                // Ignore other event types (event_msg, etc.)
            }
        }
    }

    let session_id =
        session_id.ok_or_else(|| Error::Parse("Missing session ID in session file".to_string()))?;
    let started_at =
        started_at.ok_or_else(|| Error::Parse("Missing timestamp in session file".to_string()))?;

    let working_dir = if let Some(cwd_str) = cwd {
        PathBuf::from(cwd_str)
    } else {
        PathBuf::from(".")
    };

    let agent = if let Some(model_str) = model {
        Agent::Codex { model: model_str }
    } else {
        Agent::Codex {
            model: "gpt-5-codex".to_string(),
        }
    };

    let mut execution = Execution {
        id: session_id,
        agent,
        working_dir,
        git_branch,
        started_at,
        ended_at: last_timestamp,
        summaries: instructions.map(|s| vec![s]).unwrap_or_default(),
        events,
        metrics: ExecutionMetrics::default(),
    };

    // Compute metrics from events
    execution.compute_metrics();

    Ok(execution)
}

/// Extract text from content array (handles input_text, output_text, text types)
fn extract_text_from_content(content: &serde_json::Value) -> String {
    if let Some(content_str) = content.as_str() {
        return content_str.to_string();
    }

    if let Some(content_array) = content.as_array() {
        return content_array
            .iter()
            .filter_map(|block| block.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n");
    }

    String::new()
}

/// Extract text from summary array
fn extract_summary_text(summary: &serde_json::Value) -> String {
    if let Some(summary_array) = summary.as_array() {
        return summary_array
            .iter()
            .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join(" ");
    }

    "[Reasoning]".to_string()
}

/// Parse timestamp from ISO-8601 string
fn parse_timestamp(ts: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| Error::Parse(format!("Invalid timestamp '{}': {}", ts, e)))
}

fn derive_status(exit_code: Option<i32>, is_error: Option<bool>) -> ToolStatus {
    if let Some(code) = exit_code {
        return if code == 0 {
            ToolStatus::Success
        } else {
            ToolStatus::Failure
        };
    }

    if let Some(err) = is_error {
        return if err {
            ToolStatus::Failure
        } else {
            ToolStatus::Success
        };
    }

    // If no signals provided, default to success to avoid Unknown.
    ToolStatus::Success
}
