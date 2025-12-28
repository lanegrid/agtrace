use std::fmt;

use crate::presentation::formatters::time;
use crate::presentation::view_models::{
    TuiScreenViewModel, ViewMode, WatchEventViewModel, WatchTargetViewModel,
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
            WatchEventViewModel::StreamUpdate { screen } => {
                writeln!(
                    f,
                    "update {} events={} turns={}",
                    screen.dashboard.session_id,
                    screen.timeline.displayed_count,
                    screen.status_bar.turn_count
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
            WatchEventViewModel::StreamUpdate { screen } => {
                // Show summary of events received
                let event_count = screen.timeline.displayed_count;
                let event_summary = if event_count == 1 {
                    "1 event".to_string()
                } else {
                    format!("{} events", event_count)
                };

                let total_tokens = screen.dashboard.context_total;

                let usage_str = format!(
                    "{} / {}",
                    format_with_commas(total_tokens),
                    screen
                        .dashboard
                        .context_limit
                        .map(format_with_commas)
                        .unwrap_or_else(|| "?".to_string())
                );

                writeln!(
                    f,
                    "{} {} | Turn {} | Tokens: {}",
                    "📝".green(),
                    event_summary,
                    screen.status_bar.turn_count,
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
            WatchEventViewModel::StreamUpdate { screen } => self.render_stream_update(f, screen),
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
            && let WatchEventViewModel::StreamUpdate { screen } = self.event
        {
            writeln!(f, "\n{}", "Debug Info:".dimmed())?;
            if let Some(project_root) = &screen.dashboard.project_root {
                writeln!(f, "  Project root: {}", project_root)?;
            }
            if let Some(log_path) = &screen.dashboard.log_path {
                writeln!(f, "  Log path: {}", log_path)?;
            }
            writeln!(f, "  Model: {:?}", screen.dashboard.model)?;
            writeln!(f, "  Event count: {}", screen.status_bar.event_count)?;
        }

        Ok(())
    }

    fn render_stream_update(
        &self,
        f: &mut fmt::Formatter,
        screen: &TuiScreenViewModel,
    ) -> fmt::Result {
        // Header
        writeln!(f, "\n{} Stream Update", "📝".green().bold())?;

        // Show project root if available, otherwise log path
        if let Some(project_root) = &screen.dashboard.project_root {
            writeln!(f, "  Project: {}", project_root.yellow())?;
            if let Some(log_path) = &screen.dashboard.log_path {
                writeln!(f, "    {}", log_path.dimmed())?;
            }
        } else if let Some(log_path) = &screen.dashboard.log_path {
            writeln!(f, "  Log: {}", log_path.dimmed())?;
        }

        writeln!(
            f,
            "  Session: {} | Turn {} | {} events",
            screen
                .dashboard
                .session_id
                .chars()
                .take(8)
                .collect::<String>()
                .yellow(),
            screen.status_bar.turn_count,
            screen.timeline.displayed_count
        )?;

        // Token usage
        let total_tokens = screen.dashboard.context_total;

        let usage_pct = screen
            .dashboard
            .context_usage_pct
            .map(|pct| (pct * 100.0) as u32);

        write!(f, "  Tokens: {}", format_with_commas(total_tokens).cyan())?;
        if let Some(limit) = screen.dashboard.context_limit {
            write!(f, " / {}", format_with_commas(limit))?;
        }
        if let Some(pct) = usage_pct {
            write!(f, " ({}%)", pct)?;
        }
        writeln!(f)?;

        // Show recent events (up to 5 in standard mode, all in verbose)
        let max_events = if matches!(self.mode, ViewMode::Verbose) {
            screen.timeline.events.len()
        } else {
            5.min(screen.timeline.events.len())
        };

        if !screen.timeline.events.is_empty() {
            writeln!(f, "\n  Recent events:")?;
            for event in screen.timeline.events.iter().take(max_events) {
                let timestamp = time::format_time(event.timestamp);
                writeln!(
                    f,
                    "    {} {} {}",
                    timestamp.dimmed(),
                    event.icon,
                    event.description
                )?;
            }

            if screen.timeline.events.len() > max_events {
                writeln!(
                    f,
                    "    {} ({} more events not shown)",
                    "...".dimmed(),
                    screen.timeline.events.len() - max_events
                )?;
            }
        }

        Ok(())
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
