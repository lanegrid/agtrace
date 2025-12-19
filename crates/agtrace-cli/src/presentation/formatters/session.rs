// NOTE: Compact View Philosophy
//
// Why collapse tool chains into single lines showing inputs (not outputs)?
// - Long sessions (100+ events) are hard to scan in timeline mode
// - Shows *what was asked* (command, pattern, file), not results
// - Preserves execution sequence while drastically reducing visual noise
// - Bottlenecks and loops become immediately visible via duration highlights
// - User interprets intent from facts: `Edit(schema.rs x4)` could be iteration or being stuck
// - Trade-off: Less readable for detailed debugging, but enables quick pattern recognition

use super::event::EventView;
use super::token::TokenUsageView;
use super::{FormatOptions, TokenSummaryDisplay};
use agtrace_engine::{AgentSession, AgentStep, AgentTurn};
use agtrace_types::AgentEvent;
use chrono::{DateTime, Utc};
use owo_colors::OwoColorize;
use std::fmt;

// ============================================================================
// Timeline View
// ============================================================================

/// View for displaying events in timeline format
pub struct TimelineView<'a> {
    pub events: &'a [AgentEvent],
    pub options: &'a FormatOptions,
    pub project_root: Option<&'a std::path::Path>,
}

impl<'a> fmt::Display for TimelineView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.events.is_empty() {
            return writeln!(f, "No events to display");
        }

        let session_start = self.events.first().map(|e| e.timestamp);
        let mut turn_count = 0;

        for event in self.events {
            // Skip TokenUsage events in timeline display (shown in summary)
            if matches!(event.payload, agtrace_types::EventPayload::TokenUsage(_)) {
                continue;
            }

            // Use EventView to format each event
            let view = EventView {
                event,
                options: self.options,
                session_start,
                turn_context: turn_count,
                project_root: self.project_root,
            };

            writeln!(f, "{}", view)?;

            // Increment turn count after processing user messages
            if matches!(event.payload, agtrace_types::EventPayload::User(_)) {
                turn_count += 1;
            }
        }

        // Add session summary
        writeln!(
            f,
            "{}",
            TimelineSummary {
                events: self.events,
                options: self.options,
            }
        )?;

        Ok(())
    }
}

struct TimelineSummary<'a> {
    events: &'a [AgentEvent],
    options: &'a FormatOptions,
}

impl<'a> fmt::Display for TimelineSummary<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.events.is_empty() {
            return Ok(());
        }

        let summary = summarize_events(self.events);
        let enable_color = self.options.enable_color;

        if enable_color {
            writeln!(f, "{}", "---".bright_black())?;
            writeln!(f, "{}", "Session Summary:".bright_white().bold())?;
        } else {
            writeln!(f, "---")?;
            writeln!(f, "Session Summary:")?;
        }

        // Event counts
        if enable_color {
            writeln!(
                f,
                "  {}: {}",
                "Events".cyan(),
                summary.event_counts.total.to_string().bright_white()
            )?;
            writeln!(
                f,
                "    User messages: {}",
                summary.event_counts.user_messages.to_string().green()
            )?;
            writeln!(
                f,
                "    Assistant messages: {}",
                summary.event_counts.assistant_messages.to_string().blue()
            )?;
            writeln!(
                f,
                "    Tool calls: {}",
                summary.event_counts.tool_calls.to_string().yellow()
            )?;
            writeln!(
                f,
                "    Reasoning blocks: {}",
                summary.event_counts.reasoning_blocks.to_string().cyan()
            )?;
        } else {
            writeln!(f, "  Events: {}", summary.event_counts.total)?;
            writeln!(
                f,
                "    User messages: {}",
                summary.event_counts.user_messages
            )?;
            writeln!(
                f,
                "    Assistant messages: {}",
                summary.event_counts.assistant_messages
            )?;
            writeln!(f, "    Tool calls: {}", summary.event_counts.tool_calls)?;
            writeln!(
                f,
                "    Reasoning blocks: {}",
                summary.event_counts.reasoning_blocks
            )?;
        }

        // Token summary
        let total_tokens = summary.token_stats.input + summary.token_stats.output;
        if total_tokens > 0 {
            let token_summary = TokenSummaryDisplay {
                input: summary.token_stats.input,
                output: summary.token_stats.output,
                cache_creation: summary.token_stats.cache_creation,
                cache_read: summary.token_stats.cache_read,
                total: total_tokens,
                limit: None,
                model: None,
                compaction_buffer_pct: None,
            };

            writeln!(f)?;
            write!(
                f,
                "{}",
                TokenUsageView {
                    summary: &token_summary,
                    options: self.options,
                }
            )?;
        }

        // Duration
        if let Some(duration) = &summary.duration {
            if enable_color {
                writeln!(
                    f,
                    "  {}: {}m {}s",
                    "Duration".cyan(),
                    duration.minutes,
                    duration.seconds
                )?;
            } else {
                writeln!(f, "  Duration: {}m {}s", duration.minutes, duration.seconds)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct TimelineSessionSummary {
    event_counts: TimelineEventCounts,
    token_stats: TimelineTokenStats,
    duration: Option<TimelineDuration>,
}

#[derive(Debug, Clone)]
struct TimelineTokenStats {
    input: i32,
    output: i32,
    cache_creation: i32,
    cache_read: i32,
}

#[derive(Debug, Clone)]
struct TimelineDuration {
    minutes: i64,
    seconds: i64,
}

#[derive(Debug, Clone)]
struct TimelineEventCounts {
    total: usize,
    user_messages: usize,
    assistant_messages: usize,
    tool_calls: usize,
    reasoning_blocks: usize,
}

fn summarize_events(events: &[AgentEvent]) -> TimelineSessionSummary {
    use agtrace_types::EventPayload;

    let mut user_count = 0;
    let mut assistant_count = 0;
    let mut tool_call_count = 0;
    let mut reasoning_count = 0;

    let mut total_input = 0i32;
    let mut total_output = 0i32;
    let mut total_cache_creation = 0i32;
    let mut total_cache_read = 0i32;

    for event in events {
        match &event.payload {
            EventPayload::User(_) => user_count += 1,
            EventPayload::Message(_) => assistant_count += 1,
            EventPayload::ToolCall(_) => tool_call_count += 1,
            EventPayload::Reasoning(_) => reasoning_count += 1,
            EventPayload::TokenUsage(token) => {
                total_input += token.input_tokens;
                total_output += token.output_tokens;
                if let Some(details) = &token.details {
                    if let Some(cache_creation) = details.cache_creation_input_tokens {
                        total_cache_creation += cache_creation;
                    }
                    if let Some(cache_read) = details.cache_read_input_tokens {
                        total_cache_read += cache_read;
                    }
                }
            }
            EventPayload::ToolResult(_) | EventPayload::Notification(_) => {}
        }
    }

    let duration = if let (Some(first), Some(last)) = (events.first(), events.last()) {
        let first_ts = first.timestamp;
        let last_ts = last.timestamp;
        let duration = last_ts.signed_duration_since(first_ts);
        Some(TimelineDuration {
            minutes: duration.num_minutes(),
            seconds: duration.num_seconds() % 60,
        })
    } else {
        None
    };

    TimelineSessionSummary {
        event_counts: TimelineEventCounts {
            total: events
                .iter()
                .filter(|e| !matches!(e.payload, EventPayload::TokenUsage(_)))
                .count(),
            user_messages: user_count,
            assistant_messages: assistant_count,
            tool_calls: tool_call_count,
            reasoning_blocks: reasoning_count,
        },
        token_stats: TimelineTokenStats {
            input: total_input,
            output: total_output,
            cache_creation: total_cache_creation,
            cache_read: total_cache_read,
        },
        duration,
    }
}

// ============================================================================
// Compact View
// ============================================================================

/// View for displaying a session in compact format
pub struct CompactView<'a> {
    pub session: &'a AgentSession,
    pub options: &'a FormatOptions,
}

impl<'a> fmt::Display for CompactView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.session.turns.is_empty() {
            let msg = "No turns to display";
            if self.options.enable_color {
                return writeln!(f, "{}", msg.bright_black());
            } else {
                return writeln!(f, "{}", msg);
            }
        }

        let session_start = if self.options.relative_time {
            Some(self.session.start_time)
        } else {
            None
        };

        for turn in &self.session.turns {
            write!(
                f,
                "{}",
                TurnView {
                    turn,
                    session_start,
                    options: self.options,
                }
            )?;
        }

        Ok(())
    }
}

struct TurnView<'a> {
    turn: &'a AgentTurn,
    session_start: Option<DateTime<Utc>>,
    options: &'a FormatOptions,
}

impl<'a> fmt::Display for TurnView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time_display = format_time(self.session_start, self.turn.timestamp);
        let dur_placeholder = "   -   ";

        // Format user message
        let user_text = if let Some(max_len) = self.options.truncate_text {
            truncate_text(&self.turn.user.content.text, max_len)
        } else {
            self.turn.user.content.text.clone()
        };

        let line = if self.options.enable_color {
            format!(
                "{} {} User: \"{}\"",
                time_display.dimmed(),
                dur_placeholder.dimmed(),
                user_text
            )
        } else {
            format!(
                "{} {} User: \"{}\"",
                time_display, dur_placeholder, user_text
            )
        };
        writeln!(f, "{}", line)?;

        // Format steps
        for step in &self.turn.steps {
            write!(
                f,
                "{}",
                StepView {
                    step,
                    session_start: self.session_start,
                    options: self.options,
                }
            )?;
        }

        Ok(())
    }
}

struct StepView<'a> {
    step: &'a AgentStep,
    session_start: Option<DateTime<Utc>>,
    options: &'a FormatOptions,
}

impl<'a> fmt::Display for StepView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time_display = format_time(self.session_start, self.step.timestamp);

        // Calculate duration from timestamp diff (if available)
        // For now, use a placeholder
        let duration_ms = 0i64;

        // Format step content
        let content = format_step_content(self.step, self.options.enable_color);

        // Format duration with color coding if enabled
        let dur_str = if self.options.enable_color {
            format_duration_colored(duration_ms)
        } else {
            format_duration(duration_ms.max(0) as u64)
        };

        let time_dimmed = if self.options.enable_color {
            time_display.dimmed().to_string()
        } else {
            time_display
        };

        writeln!(f, "{} {} {}", time_dimmed, dur_str, content)?;
        Ok(())
    }
}

fn format_step_content(step: &AgentStep, enable_color: bool) -> String {
    if let Some(msg) = &step.message {
        let text = truncate_text(&msg.content.text, 80);
        if enable_color {
            format!("{} {}", "💬".cyan(), text)
        } else {
            format!("Msg: {}", text)
        }
    } else if let Some(reasoning) = &step.reasoning {
        let text = truncate_text(&reasoning.content.text, 50);
        if enable_color {
            format!("{} {}", "🧠".dimmed(), text.dimmed())
        } else {
            format!("Think: {}", text)
        }
    } else if !step.tools.is_empty() {
        format_tool_chain(&step.tools, enable_color)
    } else {
        "".to_string()
    }
}

fn format_tool_chain(tools: &[agtrace_engine::ToolExecution], enable_color: bool) -> String {
    let chain: Vec<String> = tools
        .iter()
        .map(|t| {
            let name = &t.call.content.name;
            let args_summary = format_tool_args(name, &t.call.content.arguments);
            let status_indicator = if t.is_error {
                if enable_color {
                    "✗"
                } else {
                    "ERR"
                }
            } else if enable_color {
                "✓"
            } else {
                "OK"
            };

            if enable_color {
                if t.is_error {
                    format!("{}{} {}", name.red(), args_summary, status_indicator.red())
                } else {
                    format!("{}{} {}", name, args_summary, status_indicator.green())
                }
            } else {
                format!("{}{} {}", name, args_summary, status_indicator)
            }
        })
        .collect();

    if enable_color {
        format!("{} {}", "🔧".yellow(), chain.join(" → "))
    } else {
        format!("Tools: {}", chain.join(" -> "))
    }
}

fn format_tool_args(_name: &str, args: &serde_json::Value) -> String {
    if let Some(obj) = args.as_object() {
        if let Some(path) = obj.get("path").or_else(|| obj.get("file_path")) {
            if let Some(path_str) = path.as_str() {
                return format!("({})", truncate_text(path_str, 30));
            }
        }
        if let Some(cmd) = obj.get("command") {
            if let Some(cmd_str) = cmd.as_str() {
                return format!("({})", truncate_text(cmd_str, 30));
            }
        }
        if let Some(pattern) = obj.get("pattern") {
            if let Some(pat_str) = pattern.as_str() {
                return format!("({})", truncate_text(pat_str, 30));
            }
        }
    }
    "".to_string()
}

// ============================================================================
// Helper Functions
// ============================================================================

fn format_time(session_start: Option<DateTime<Utc>>, timestamp: DateTime<Utc>) -> String {
    if let Some(start) = session_start {
        let duration = timestamp.signed_duration_since(start);
        let seconds = duration.num_seconds();
        if seconds < 60 {
            format!("[+{:02}s  ]", seconds)
        } else {
            let minutes = seconds / 60;
            let secs = seconds % 60;
            format!("[+{}m {:02}s]", minutes, secs)
        }
    } else {
        let ts = timestamp.with_timezone(&chrono::Local).format("%H:%M:%S");
        format!("[{}]", ts)
    }
}

fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{:4}ms", ms)
    } else if ms < 60_000 {
        format!("{:4.1}s ", ms as f64 / 1000.0)
    } else {
        let minutes = ms / 60_000;
        let seconds = (ms % 60_000) / 1000;
        format!("{}m{:02}s", minutes, seconds)
    }
}

fn format_duration_colored(ms: i64) -> String {
    let ms = ms.max(0) as u64;
    let dur_str = format_duration(ms);

    if ms >= 30_000 {
        dur_str.red().to_string()
    } else if ms >= 10_000 {
        dur_str.yellow().to_string()
    } else {
        dur_str.dimmed().to_string()
    }
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        let mut boundary = max_len;
        while boundary > 0 && !text.is_char_boundary(boundary) {
            boundary -= 1;
        }
        format!("{}...", &text[..boundary])
    }
}

/// Calculate token summary from an AgentSession
pub fn calculate_token_summary(session: &AgentSession) -> TokenSummaryDisplay {
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
        compaction_buffer_pct: None,
    }
}
