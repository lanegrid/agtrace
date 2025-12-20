use crate::presentation::formatters::{text, tool, FormatOptions, TokenSummaryDisplay};
use crate::presentation::view_models::{SessionViewModel, StepViewModel, TurnViewModel};
use chrono::{DateTime, Utc};
use owo_colors::OwoColorize;
use std::fmt;

pub struct CompactSessionView<'a> {
    pub session: &'a SessionViewModel,
    pub options: &'a FormatOptions,
}

impl<'a> fmt::Display for CompactSessionView<'a> {
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
    turn: &'a TurnViewModel,
    session_start: Option<DateTime<Utc>>,
    options: &'a FormatOptions,
}

impl<'a> fmt::Display for TurnView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time_display = format_time(self.session_start, self.turn.timestamp);
        let dur_placeholder = "   -   ";

        let user_text = if let Some(max_len) = self.options.truncate_text {
            text::truncate(&self.turn.user_message, max_len)
        } else {
            self.turn.user_message.clone()
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
    step: &'a StepViewModel,
    session_start: Option<DateTime<Utc>>,
    options: &'a FormatOptions,
}

impl<'a> fmt::Display for StepView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time_display = format_time(self.session_start, self.step.timestamp);

        let content = format_step_content(self.step, self.options.enable_color);

        let time_dimmed = if self.options.enable_color {
            time_display.dimmed().to_string()
        } else {
            time_display
        };

        let dur_str = "   -   ";

        writeln!(f, "{} {} {}", time_dimmed, dur_str, content)?;
        Ok(())
    }
}

fn format_step_content(step: &StepViewModel, enable_color: bool) -> String {
    if let Some(msg) = &step.message_text {
        let txt = text::truncate(msg, 80);
        if enable_color {
            format!("{} {}", "💬".cyan(), txt)
        } else {
            format!("Msg: {}", txt)
        }
    } else if let Some(reasoning) = &step.reasoning_text {
        let txt = text::truncate(reasoning, 50);
        if enable_color {
            format!("{} {}", "🧠".dimmed(), txt.dimmed())
        } else {
            format!("Think: {}", txt)
        }
    } else if !step.tools.is_empty() {
        format_tool_chain(&step.tools, enable_color)
    } else {
        "".to_string()
    }
}

fn format_tool_chain(
    tools: &[crate::presentation::view_models::ToolExecutionViewModel],
    enable_color: bool,
) -> String {
    let chain: Vec<String> = tools
        .iter()
        .map(|t| {
            let name = &t.name;
            let args_summary = tool::format_tool_call(&t.name, &t.arguments, None);
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

pub fn calculate_token_summary(session: &SessionViewModel) -> TokenSummaryDisplay {
    let mut total_input = 0i32;
    let mut total_output = 0i32;
    let mut total_cache_creation = 0i32;
    let mut total_cache_read = 0i32;

    for turn in &session.turns {
        for step in &turn.steps {
            if let Some(usage) = &step.usage {
                total_input += usage.input_tokens;
                total_output += usage.output_tokens;

                if let Some(cache_creation) = usage.cache_creation_tokens {
                    total_cache_creation += cache_creation;
                }
                if let Some(cache_read) = usage.cache_read_tokens {
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
        limit: None,
        model: None,
        compaction_buffer_pct: None,
    }
}
