use crate::display_model::{
    DisplayOptions, SessionDisplay, StepContent, TokenSummaryDisplay, ToolDisplay,
};
use agtrace_types::ToolStatus;
use chrono::{DateTime, Utc};
use owo_colors::OwoColorize;

// NOTE: Compact View Philosophy
//
// Why collapse tool chains into single lines showing inputs (not outputs)?
// - Long sessions (100+ events) are hard to scan in timeline mode
// - Shows *what was asked* (command, pattern, file), not results
// - Preserves execution sequence while drastically reducing visual noise
// - Bottlenecks and loops become immediately visible via duration highlights
// - User interprets intent from facts: `Edit(schema.rs x4)` could be iteration or being stuck
// - Trade-off: Less readable for detailed debugging, but enables quick pattern recognition

pub fn format_compact(display: &SessionDisplay, opts: &DisplayOptions) -> Vec<String> {
    if display.turns.is_empty() {
        let msg = "No turns to display";
        return vec![if opts.enable_color {
            format!("{}", msg.bright_black())
        } else {
            msg.to_string()
        }];
    }

    let session_start = if opts.relative_time {
        Some(display.start_time)
    } else {
        None
    };

    let mut lines = Vec::new();

    for turn in &display.turns {
        let time_display = format_time_utc(session_start, "", turn.timestamp);
        let dur_placeholder = "   -   ";

        let user_text = if let Some(max_len) = opts.truncate_text {
            truncate_text(&turn.user_text, max_len)
        } else {
            turn.user_text.clone()
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

        for step in &turn.steps {
            match &step.content {
                StepContent::Reasoning { text } => {
                    let time_display = format_time_utc(session_start, "", step.timestamp);
                    let reasoning_text = if let Some(max_len) = opts.truncate_text {
                        truncate_text(text, max_len)
                    } else {
                        text.clone()
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
                StepContent::Tools { executions } => {
                    let duration_ms = executions
                        .iter()
                        .filter_map(|t| t.duration_ms)
                        .max()
                        .unwrap_or(0);
                    let dur_str = format_duration(duration_ms);

                    let tools_display = format_tool_executions(executions, opts.enable_color);

                    let time_display = format_time_utc(session_start, "", step.timestamp);

                    let line = if opts.enable_color {
                        let dur_colored = if duration_ms > 30000 {
                            format!("{}", dur_str.red())
                        } else if duration_ms > 10000 {
                            format!("{}", dur_str.yellow())
                        } else {
                            format!("{}", dur_str.bright_black())
                        };
                        format!(
                            "{} {} {}",
                            time_display.bright_black(),
                            dur_colored,
                            tools_display.cyan()
                        )
                    } else {
                        format!("{} {} {}", time_display, dur_str, tools_display)
                    };
                    lines.push(line);
                }
                StepContent::Message { text } => {
                    let time_display = format_time_utc(session_start, "", step.timestamp);

                    let message_text = if let Some(max_len) = opts.truncate_text {
                        truncate_text(text, max_len)
                    } else {
                        text.clone()
                    };

                    let line = if opts.enable_color {
                        format!(
                            "{} {} Assistant: \"{}\"",
                            time_display.bright_black(),
                            dur_placeholder.bright_black(),
                            message_text.blue()
                        )
                    } else {
                        format!(
                            "{} {} Assistant: \"{}\"",
                            time_display, dur_placeholder, message_text
                        )
                    };
                    lines.push(line);
                }
            }
        }
    }

    lines
}

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
            let cache_creation_str = format_token_count(summary.cache_creation as u64);
            let cache_read_str = format_token_count(summary.cache_read as u64);
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
                        lines.push(format!(
                            "  {} Creation: {}",
                            "↳".dimmed(),
                            cache_creation_str
                        ));
                    }
                    if summary.cache_read > 0 {
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

                if summary.cache_creation > 0 || summary.cache_read > 0 {
                    let cache_total = summary.cache_creation + summary.cache_read;
                    let cache_pct = (cache_total as f64 / limit as f64) * 100.0;
                    let cache_total_str = format_token_count(cache_total as u64);
                    lines.push(format!(
                        "⛁ Cache:   {} ({:.1}%)",
                        cache_total_str, cache_pct
                    ));
                    if summary.cache_creation > 0 {
                        lines.push(format!("  ↳ Creation: {}", cache_creation_str));
                    }
                    if summary.cache_read > 0 {
                        lines.push(format!("  ↳ Read:     {}", cache_read_str));
                    }
                }

                lines.push(format!("⛶ Free:    {} ({:.1}%)", free_str, free_pct));
            }

            if total_pct >= 95.0 {
                let warning = "⚠️  Critical - Start new session immediately";
                lines.push(if opts.enable_color {
                    format!("{}", warning.red().bold())
                } else {
                    warning.to_string()
                });
            } else if total_pct >= 80.0 {
                let warning = "⚠️  Warning - Consider wrapping up soon";
                lines.push(if opts.enable_color {
                    format!("{}", warning.yellow())
                } else {
                    warning.to_string()
                });
            }
        }
    } else {
        // Display without limit info
        let model_name = summary.model.as_deref().unwrap_or("unknown");
        let header = format!("Token Usage ({})", model_name);
        lines.push(if opts.enable_color {
            format!("{}", header.bright_black())
        } else {
            header
        });

        let total_str = format_token_count(summary.total as u64);
        let input_str = format_token_count(summary.input as u64);
        let output_str = format_token_count(summary.output as u64);

        lines.push(format!("Total: {}", total_str));

        if opts.enable_color {
            lines.push(format!("{} Input:  {}", "⛁".cyan(), input_str));
            lines.push(format!("{} Output: {}", "⛁".cyan(), output_str));

            if summary.cache_creation > 0 || summary.cache_read > 0 {
                let cache_creation_str = format_token_count(summary.cache_creation as u64);
                let cache_read_str = format_token_count(summary.cache_read as u64);

                if summary.cache_creation > 0 {
                    lines.push(format!(
                        "{} Cache Creation: {}",
                        "⛁".cyan(),
                        cache_creation_str
                    ));
                }
                if summary.cache_read > 0 {
                    lines.push(format!("{} Cache Read: {}", "⛁".cyan(), cache_read_str));
                }
            }
        } else {
            lines.push(format!("⛁ Input:  {}", input_str));
            lines.push(format!("⛁ Output: {}", output_str));

            if summary.cache_creation > 0 {
                let cache_creation_str = format_token_count(summary.cache_creation as u64);
                lines.push(format!("⛁ Cache Creation: {}", cache_creation_str));
            }
            if summary.cache_read > 0 {
                let cache_read_str = format_token_count(summary.cache_read as u64);
                lines.push(format!("⛁ Cache Read: {}", cache_read_str));
            }
        }
    }

    lines
}

fn format_time_utc(
    session_start: Option<DateTime<Utc>>,
    _text: &str,
    timestamp: DateTime<Utc>,
) -> String {
    if let Some(start) = session_start {
        let duration = timestamp.signed_duration_since(start);
        let seconds = duration.num_seconds();
        if seconds < 60 {
            format!("[+{:02}:{:02}]", 0, seconds)
        } else {
            let minutes = seconds / 60;
            let secs = seconds % 60;
            format!("[+{:02}:{:02}]", minutes, secs)
        }
    } else {
        let ts_str = timestamp.to_rfc3339();
        if ts_str.len() >= 19 {
            format!("[{}]", &ts_str[11..19])
        } else {
            "[+00:00]".to_string()
        }
    }
}

fn format_duration(duration_ms: u64) -> String {
    if duration_ms >= 1000 {
        format!("{:2}s    ", duration_ms / 1000)
    } else if duration_ms > 0 {
        format!("{:3}ms  ", duration_ms)
    } else {
        "   -   ".to_string()
    }
}

fn format_tool_executions(tools: &[ToolDisplay], enable_color: bool) -> String {
    let parts: Vec<String> = tools
        .iter()
        .map(|tool| {
            let status_indicator = format_tool_status(&tool.status, enable_color);

            let tool_display = if !tool.arguments_summary.is_empty() {
                format!("{}({})", tool.name, tool.arguments_summary)
            } else {
                tool.name.clone()
            };
            format!("{}{}", status_indicator, tool_display)
        })
        .collect();

    if parts.is_empty() {
        "no tools".to_string()
    } else {
        parts.join(" → ")
    }
}

fn format_tool_status(status: &ToolStatus, enable_color: bool) -> String {
    match status {
        ToolStatus::Success => {
            if enable_color {
                format!("{} ", "✓".green())
            } else {
                "✓ ".to_string()
            }
        }
        ToolStatus::Error => {
            if enable_color {
                format!("{} ", "✗".red())
            } else {
                "✗ ".to_string()
            }
        }
        ToolStatus::InProgress => {
            if enable_color {
                format!("{} ", "⏳".yellow())
            } else {
                "⏳ ".to_string()
            }
        }
        ToolStatus::Unknown => {
            if enable_color {
                format!("{} ", "?".bright_black())
            } else {
                "? ".to_string()
            }
        }
    }
}

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let chars: Vec<char> = text.chars().take(max_len - 3).collect();
        format!("{}...", chars.iter().collect::<String>())
    }
}

fn create_progress_bar(percentage: f64) -> String {
    let bar_width = 10;
    let filled = ((percentage / 100.0) * bar_width as f64).round() as usize;
    let filled = filled.min(bar_width);
    let empty = bar_width - filled;

    format!("{}{}", "⛁ ".repeat(filled), "⛶ ".repeat(empty))
}

fn format_token_count(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{}k", tokens / 1_000)
    } else {
        format!("{}", tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("hello", 10), "hello");
        assert_eq!(truncate_text("hello world", 8), "hello...");
    }

    #[test]
    fn test_format_token_count() {
        assert_eq!(format_token_count(500), "500");
        assert_eq!(format_token_count(5_000), "5k");
        assert_eq!(format_token_count(1_500_000), "1.5M");
    }

    #[test]
    fn test_create_progress_bar() {
        let bar = create_progress_bar(50.0);
        assert!(bar.contains("⛁"));
        assert!(bar.contains("⛶"));

        // Test 0%
        let bar_empty = create_progress_bar(0.0);
        assert_eq!(bar_empty, "⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ⛶ ");

        // Test 100%
        let bar_full = create_progress_bar(100.0);
        assert_eq!(bar_full, "⛁ ⛁ ⛁ ⛁ ⛁ ⛁ ⛁ ⛁ ⛁ ⛁ ");
    }
}
