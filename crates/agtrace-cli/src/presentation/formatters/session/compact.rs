// NOTE: Compact View Philosophy
//
// Why collapse tool chains into single lines showing inputs (not outputs)?
// - Long sessions (100+ events) are hard to scan in timeline mode
// - Shows *what was asked* (command, pattern, file), not results
// - Preserves execution sequence while drastically reducing visual noise
// - Bottlenecks and loops become immediately visible via duration highlights
// - User interprets intent from facts: `Edit(schema.rs x4)` could be iteration or being stuck
// - Trade-off: Less readable for detailed debugging, but enables quick pattern recognition

use crate::presentation::formatters::{DisplayOptions, TokenSummaryDisplay};
use agtrace_engine::{AgentSession, AgentStep, AgentTurn};
use chrono::{DateTime, Utc};
use owo_colors::OwoColorize;

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

/// Format an AgentSession in compact view
/// This function directly processes the domain model without intermediate conversion
pub fn format_compact(session: &AgentSession, opts: &DisplayOptions) -> Vec<String> {
    if session.turns.is_empty() {
        let msg = "No turns to display";
        return vec![if opts.enable_color {
            format!("{}", msg.bright_black())
        } else {
            msg.to_string()
        }];
    }

    let session_start = if opts.relative_time {
        Some(session.start_time)
    } else {
        None
    };

    let mut lines = Vec::new();

    for turn in &session.turns {
        format_turn(&mut lines, turn, session_start, opts);
    }

    lines
}

fn format_turn(
    lines: &mut Vec<String>,
    turn: &AgentTurn,
    session_start: Option<DateTime<Utc>>,
    opts: &DisplayOptions,
) {
    let time_display = format_time(session_start, turn.timestamp);
    let dur_placeholder = "   -   ";

    // Format user message
    let user_text = if let Some(max_len) = opts.truncate_text {
        truncate_text(&turn.user.content.text, max_len)
    } else {
        turn.user.content.text.clone()
    };

    let line = if opts.enable_color {
        format!(
            "{} {} User: \"{}\"",
            time_display.bright_black(),
            dur_placeholder.bright_black(),
            user_text.green()
        )
    } else {
        format!(
            "{} {} User: \"{}\"",
            time_display, dur_placeholder, user_text
        )
    };
    lines.push(line);

    // Format steps
    for step in &turn.steps {
        format_step(lines, step, session_start, opts);
    }
}

fn format_step(
    lines: &mut Vec<String>,
    step: &AgentStep,
    session_start: Option<DateTime<Utc>>,
    opts: &DisplayOptions,
) {
    let time_display = format_time(session_start, step.timestamp);
    let dur_placeholder = "   -   ";

    // Reasoning
    if let Some(reasoning) = &step.reasoning {
        let reasoning_text = if let Some(max_len) = opts.truncate_text {
            truncate_text(&reasoning.content.text, max_len)
        } else {
            reasoning.content.text.clone()
        };

        let line = if opts.enable_color {
            format!(
                "{} {} Reasoning: \"{}\"",
                time_display.bright_black(),
                dur_placeholder.bright_black(),
                reasoning_text.cyan()
            )
        } else {
            format!(
                "{} {} Reasoning: \"{}\"",
                time_display, dur_placeholder, reasoning_text
            )
        };
        lines.push(line);
    }

    // Tools
    if !step.tools.is_empty() {
        let duration_ms = step
            .tools
            .iter()
            .filter_map(|t| t.duration_ms)
            .max()
            .unwrap_or(0);
        let duration_u64 = duration_ms.max(0) as u64;
        let dur_str = format_duration(duration_u64);

        let tools_display = format_tool_executions(&step.tools, opts.enable_color);

        let line = if opts.enable_color {
            let dur_colored = if duration_u64 > 30000 {
                format!("{}", dur_str.red())
            } else if duration_u64 > 10000 {
                format!("{}", dur_str.yellow())
            } else {
                dur_str
            };
            format!(
                "{} {} Tools: {}",
                time_display.bright_black(),
                dur_colored,
                tools_display
            )
        } else {
            format!("{} {} Tools: {}", time_display, dur_str, tools_display)
        };
        lines.push(line);
    }

    // Message (assistant response)
    if let Some(message) = &step.message {
        let msg_text = if let Some(max_len) = opts.truncate_text {
            truncate_text(&message.content.text, max_len)
        } else {
            message.content.text.clone()
        };

        let line = if opts.enable_color {
            format!(
                "{} {} Message: \"{}\"",
                time_display.bright_black(),
                dur_placeholder.bright_black(),
                msg_text.blue()
            )
        } else {
            format!(
                "{} {} Message: \"{}\"",
                time_display, dur_placeholder, msg_text
            )
        };
        lines.push(line);
    }
}

fn format_tool_executions(tools: &[agtrace_engine::ToolExecution], enable_color: bool) -> String {
    tools
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
        .collect::<Vec<_>>()
        .join(" → ")
}

/// Format token summary display
pub fn format_token_summary(summary: &TokenSummaryDisplay, opts: &DisplayOptions) -> Vec<String> {
    let mut lines = Vec::new();

    if summary.total == 0 {
        return lines;
    }

    // Display token usage even without limit
    if let Some(limit) = summary.limit {
        let total_pct = (summary.total as f64 / limit as f64) * 100.0;
        let input_pct = (summary.input as f64 / limit as f64) * 100.0;
        let output_pct = (summary.output as f64 / limit as f64) * 100.0;

        let model_name = summary.model.as_deref().unwrap_or("unknown");
        let header = format!("Context Window ({})", model_name);
        lines.push(if opts.enable_color {
            format!("{}", header.bright_black())
        } else {
            header
        });

        let bar = create_progress_bar(total_pct);
        let total_str = format_token_count(summary.total as u64);
        let limit_str = format_token_count(limit);

        let color_fn: fn(&str) -> String = if total_pct >= 95.0 {
            |s: &str| s.red().to_string()
        } else if total_pct >= 80.0 {
            |s: &str| s.yellow().to_string()
        } else {
            |s: &str| s.green().to_string()
        };

        let progress_line = format!(
            "{}  {}/{} tokens ({:.1}%)",
            if opts.enable_color {
                color_fn(&bar)
            } else {
                bar
            },
            total_str,
            limit_str,
            total_pct
        );
        lines.push(progress_line);

        if total_pct >= 70.0 {
            let free_tokens = limit.saturating_sub(summary.total as u64);
            let free_pct = 100.0 - total_pct;

            let input_str = format_token_count(summary.input as u64);
            let output_str = format_token_count(summary.output as u64);
            let free_str = format_token_count(free_tokens);

            if opts.enable_color {
                lines.push(format!(
                    "{} Input:   {} ({:.1}%)",
                    "⛁".cyan(),
                    input_str,
                    input_pct
                ));
                lines.push(format!(
                    "{} Output:  {} ({:.1}%)",
                    "⛁".cyan(),
                    output_str,
                    output_pct
                ));

                if summary.cache_creation > 0 || summary.cache_read > 0 {
                    let cache_total = summary.cache_creation + summary.cache_read;
                    let cache_pct = (cache_total as f64 / limit as f64) * 100.0;
                    let cache_total_str = format_token_count(cache_total as u64);
                    lines.push(format!(
                        "{} Cache:   {} ({:.1}%)",
                        "⛁".cyan(),
                        cache_total_str,
                        cache_pct
                    ));

                    if summary.cache_creation > 0 {
                        let cache_creation_str = format_token_count(summary.cache_creation as u64);
                        lines.push(format!(
                            "  {} Creation: {}",
                            "↳".dimmed(),
                            cache_creation_str
                        ));
                    }
                    if summary.cache_read > 0 {
                        let cache_read_str = format_token_count(summary.cache_read as u64);
                        lines.push(format!("  {} Read:     {}", "↳".dimmed(), cache_read_str));
                    }
                }

                lines.push(format!(
                    "{} Free:    {} ({:.1}%)",
                    "⛶".dimmed(),
                    free_str,
                    free_pct
                ));
            } else {
                lines.push(format!("⛁ Input:   {} ({:.1}%)", input_str, input_pct));
                lines.push(format!("⛁ Output:  {} ({:.1}%)", output_str, output_pct));
                lines.push(format!("⛶ Free:    {} ({:.1}%)", free_str, free_pct));
            }

            if let Some(buffer_pct) = summary.compaction_buffer_pct {
                if buffer_pct > 0.0 {
                    let trigger_pct = 100.0 - buffer_pct;
                    let status = if input_pct >= trigger_pct {
                        if opts.enable_color {
                            "TRIGGERED".red().to_string()
                        } else {
                            "TRIGGERED".to_string()
                        }
                    } else if opts.enable_color {
                        format!("at {:.0}%", trigger_pct).dimmed().to_string()
                    } else {
                        format!("at {:.0}%", trigger_pct)
                    };
                    lines.push(format!("  Compaction buffer: {}", status));
                }
            }
        }
    } else {
        // No limit available, just show totals
        let total_str = format_token_count(summary.total as u64);
        let input_str = format_token_count(summary.input as u64);
        let output_str = format_token_count(summary.output as u64);

        lines.push(format!("Total:  {} tokens", total_str));
        lines.push(format!("Input:  {}", input_str));
        lines.push(format!("Output: {}", output_str));
    }

    lines
}

// Helper functions

fn format_time(session_start: Option<DateTime<Utc>>, timestamp: DateTime<Utc>) -> String {
    if let Some(start) = session_start {
        let duration = timestamp.signed_duration_since(start);
        let seconds = duration.num_seconds();
        if seconds < 60 {
            format!("+{}s", seconds)
        } else {
            let minutes = seconds / 60;
            let remaining_secs = seconds % 60;
            format!("+{}m{:02}s", minutes, remaining_secs)
        }
    } else {
        timestamp.format("%H:%M:%S").to_string()
    }
}

fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{:3}ms", ms)
    } else {
        format!("{:3.1}s ", ms as f64 / 1000.0)
    }
}

fn truncate_text(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let chars: Vec<char> = s.chars().take(max_len - 3).collect();
        format!("{}...", chars.iter().collect::<String>())
    }
}

fn format_tool_args(name: &str, args: &serde_json::Value) -> String {
    match name {
        "Read" | "Edit" | "Write" => {
            if let Some(path) = args.get("file_path").or_else(|| args.get("path")) {
                if let Some(path_str) = path.as_str() {
                    return format!("({})", path_str);
                }
            }
        }
        "Bash" => {
            if let Some(cmd) = args.get("command") {
                if let Some(cmd_str) = cmd.as_str() {
                    let truncated = if cmd_str.len() > 50 {
                        format!("{}...", &cmd_str[..50])
                    } else {
                        cmd_str.to_string()
                    };
                    return format!("({})", truncated);
                }
            }
        }
        "Grep" => {
            if let Some(pattern) = args.get("pattern") {
                if let Some(pat_str) = pattern.as_str() {
                    return format!("(\"{}\")", pat_str);
                }
            }
        }
        _ => {}
    }
    String::new()
}

fn create_progress_bar(percentage: f64) -> String {
    let bar_width = 20;
    let filled = ((percentage / 100.0) * bar_width as f64) as usize;
    let filled = filled.min(bar_width);
    let empty = bar_width - filled;

    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

fn format_token_count(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_progress_bar() {
        assert_eq!(create_progress_bar(0.0), "[░░░░░░░░░░░░░░░░░░░░]");
        assert_eq!(create_progress_bar(50.0), "[██████████░░░░░░░░░░]");
        assert_eq!(create_progress_bar(100.0), "[████████████████████]");
    }

    #[test]
    fn test_format_token_count() {
        assert_eq!(format_token_count(500), "500");
        assert_eq!(format_token_count(1500), "1.5K");
        assert_eq!(format_token_count(1500000), "1.5M");
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("short", 10), "short");
        assert_eq!(truncate_text("this is a long text", 10), "this is...");
    }
}
