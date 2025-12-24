use std::fmt;

use crate::presentation::formatters::time;
use crate::presentation::view_models::{
    ViewMode, WatchEventViewModel, WatchStreamStateViewModel, WatchTargetViewModel,
};
use owo_colors::OwoColorize;

// --------------------------------------------------------
// Watch Event View
// --------------------------------------------------------

pub struct WatchEventView<'a> {
    event: &'a WatchEventViewModel,
    mode: ViewMode,
}

impl<'a> WatchEventView<'a> {
    pub fn new(event: &'a WatchEventViewModel, mode: ViewMode) -> Self {
        Self { event, mode }
    }
}

impl<'a> fmt::Display for WatchEventView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}

impl<'a> WatchEventView<'a> {
    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Minimal: Just event type and key info
        match self.event {
            WatchEventViewModel::Start { target } => {
                let target_str = match target {
                    WatchTargetViewModel::Provider { name, .. } => format!("provider:{}", name),
                    WatchTargetViewModel::Session { id, .. } => format!("session:{}", id),
                };
                writeln!(f, "start {}", target_str)
            }
            WatchEventViewModel::Attached { session_id } => {
                writeln!(f, "attached {}", session_id)
            }
            WatchEventViewModel::Rotated {
                old_session,
                new_session,
            } => {
                writeln!(f, "rotated {} -> {}", old_session, new_session)
            }
            WatchEventViewModel::Waiting { message } => {
                writeln!(f, "waiting {}", message)
            }
            WatchEventViewModel::StreamUpdate { state, events, .. } => {
                writeln!(
                    f,
                    "update {} events={} turns={}",
                    state.session_id,
                    events.len(),
                    state.turn_count
                )
            }
            WatchEventViewModel::Error { message, fatal } => {
                writeln!(f, "error fatal={} {}", fatal, message)
            }
        }
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Compact: Single line with key details
        match self.event {
            WatchEventViewModel::Start { target } => {
                let (type_str, name, path) = match target {
                    WatchTargetViewModel::Provider { name, log_root } => {
                        ("Provider", name.as_str(), log_root.display().to_string())
                    }
                    WatchTargetViewModel::Session { id, log_root } => {
                        ("Session", id.as_str(), log_root.display().to_string())
                    }
                };
                writeln!(
                    f,
                    "{} Watching {} {} ({})",
                    "👀".cyan(),
                    type_str.bold(),
                    name.yellow(),
                    path.dimmed()
                )
            }
            WatchEventViewModel::Attached { session_id } => {
                writeln!(
                    f,
                    "{} Attached to session {}",
                    "✨".green(),
                    session_id.yellow()
                )
            }
            WatchEventViewModel::Rotated {
                old_session,
                new_session,
            } => {
                writeln!(
                    f,
                    "{} Session rotated: {} → {}",
                    "🔄".cyan(),
                    old_session.dimmed(),
                    new_session.yellow()
                )
            }
            WatchEventViewModel::Waiting { message } => {
                writeln!(f, "{} {}", "⏳".dimmed(), message.dimmed())
            }
            WatchEventViewModel::StreamUpdate { state, events, .. } => {
                // Show summary of events received
                let event_summary = if events.len() == 1 {
                    "1 event".to_string()
                } else {
                    format!("{} events", events.len())
                };

                let total_tokens = state.current_usage.fresh_input
                    + state.current_usage.cache_creation
                    + state.current_usage.cache_read
                    + state.current_usage.output;

                let usage_str = format!(
                    "{} / {}",
                    format_with_commas(total_tokens as u64),
                    state
                        .token_limit
                        .map(format_with_commas)
                        .unwrap_or_else(|| "?".to_string())
                );

                writeln!(
                    f,
                    "{} {} | Turn {} | Tokens: {}",
                    "📝".green(),
                    event_summary,
                    state.turn_count,
                    usage_str.cyan()
                )
            }
            WatchEventViewModel::Error { message, fatal } => {
                let prefix = if *fatal { "❌ FATAL" } else { "⚠️  ERROR" };
                writeln!(f, "{}: {}", prefix.red().bold(), message)
            }
        }
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Standard: Multi-line with details
        match self.event {
            WatchEventViewModel::Start { target } => {
                let (type_str, name, path) = match target {
                    WatchTargetViewModel::Provider { name, log_root } => {
                        ("Provider", name.as_str(), log_root.display().to_string())
                    }
                    WatchTargetViewModel::Session { id, log_root } => {
                        ("Session", id.as_str(), log_root.display().to_string())
                    }
                };
                writeln!(f, "\n{}", "═".repeat(60).dimmed())?;
                writeln!(f, "{} Watching {}", "👀".cyan(), type_str.bold())?;
                writeln!(f, "  Name: {}", name.yellow())?;
                writeln!(f, "  Path: {}", path.dimmed())?;
                writeln!(f, "{}", "═".repeat(60).dimmed())
            }
            WatchEventViewModel::Attached { session_id } => {
                writeln!(f, "\n{} Attached to session", "✨".green().bold())?;
                writeln!(f, "  Session ID: {}", session_id.yellow())
            }
            WatchEventViewModel::Rotated {
                old_session,
                new_session,
            } => {
                writeln!(f, "\n{} Session rotated", "🔄".cyan().bold())?;
                writeln!(f, "  From: {}", old_session.dimmed())?;
                writeln!(f, "  To:   {}", new_session.yellow())
            }
            WatchEventViewModel::Waiting { message } => {
                writeln!(f, "{} {}", "⏳".dimmed(), message.dimmed())
            }
            WatchEventViewModel::StreamUpdate {
                state,
                events,
                turns,
            } => self.render_stream_update(f, state, events, turns.as_deref()),
            WatchEventViewModel::Error { message, fatal } => {
                let prefix = if *fatal {
                    "❌ FATAL ERROR"
                } else {
                    "⚠️  ERROR"
                };
                writeln!(f, "\n{}", prefix.red().bold())?;
                writeln!(f, "  {}", message)
            }
        }
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Verbose: All details including raw data
        self.render_standard(f)?;

        // Add extra debug info in verbose mode
        if matches!(self.mode, ViewMode::Verbose)
            && let WatchEventViewModel::StreamUpdate { state, .. } = self.event
        {
            writeln!(f, "\n{}", "Debug Info:".dimmed())?;
            writeln!(f, "  Project root: {:?}", state.project_root)?;
            writeln!(f, "  Model: {:?}", state.model)?;
            writeln!(f, "  Event count: {}", state.event_count)?;
        }

        Ok(())
    }

    fn render_stream_update(
        &self,
        f: &mut fmt::Formatter,
        state: &WatchStreamStateViewModel,
        events: &[crate::presentation::view_models::EventViewModel],
        _turns: Option<&[crate::presentation::view_models::TurnUsageViewModel]>,
    ) -> fmt::Result {
        use crate::presentation::view_models::EventPayloadViewModel;

        // Header
        writeln!(f, "\n{} Stream Update", "📝".green().bold())?;
        writeln!(
            f,
            "  Session: {} | Turn {} | {} events",
            state
                .session_id
                .chars()
                .take(8)
                .collect::<String>()
                .yellow(),
            state.turn_count,
            events.len()
        )?;

        // Token usage
        let total_tokens = state.current_usage.fresh_input
            + state.current_usage.cache_creation
            + state.current_usage.cache_read
            + state.current_usage.output;

        let usage_pct = state.token_limit.map(|limit| {
            if limit > 0 {
                (total_tokens as f64 / limit as f64 * 100.0) as u32
            } else {
                0
            }
        });

        write!(
            f,
            "  Tokens: {}",
            format_with_commas(total_tokens as u64).cyan()
        )?;
        if let Some(limit) = state.token_limit {
            write!(f, " / {}", format_with_commas(limit))?;
        }
        if let Some(pct) = usage_pct {
            write!(f, " ({}%)", pct)?;
        }
        writeln!(f)?;

        // Show recent events (up to 5 in standard mode, all in verbose)
        let max_events = if matches!(self.mode, ViewMode::Verbose) {
            events.len()
        } else {
            5.min(events.len())
        };

        if !events.is_empty() {
            writeln!(f, "\n  Recent events:")?;
            for event in events.iter().take(max_events) {
                let timestamp = time::format_time(event.timestamp);
                let (emoji, description) = match &event.payload {
                    EventPayloadViewModel::User { text } => {
                        ("💬", format!("User: {}", truncate(text, 60)))
                    }
                    EventPayloadViewModel::Reasoning { text } => {
                        ("🤔", format!("Thinking: {}", truncate(text, 60)))
                    }
                    EventPayloadViewModel::Message { text } => {
                        ("📤", format!("Response: {}", truncate(text, 60)))
                    }
                    EventPayloadViewModel::ToolCall { name, .. } => {
                        ("🔧", format!("Tool: {}", name))
                    }
                    EventPayloadViewModel::ToolResult { output, is_error } => {
                        if *is_error {
                            ("❌", format!("Tool error: {}", truncate(output, 60)))
                        } else {
                            ("✅", format!("Tool result: {}", truncate(output, 60)))
                        }
                    }
                    EventPayloadViewModel::TokenUsage { .. } => ("📊", "Token usage".to_string()),
                    EventPayloadViewModel::Notification { text, .. } => {
                        ("ℹ️", format!("Notification: {}", truncate(text, 60)))
                    }
                };

                writeln!(f, "    {} {} {}", timestamp.dimmed(), emoji, description)?;
            }

            if events.len() > max_events {
                writeln!(
                    f,
                    "    {} ({} more events not shown)",
                    "...".dimmed(),
                    events.len() - max_events
                )?;
            }
        }

        Ok(())
    }
}

fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}

fn format_with_commas(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();

    for (count, c) in s.chars().rev().enumerate() {
        if count > 0 && count % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }

    result.chars().rev().collect()
}

// Implement Display for WatchEventViewModel directly for convenience
impl fmt::Display for WatchEventViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", WatchEventView::new(self, ViewMode::Standard))
    }
}
