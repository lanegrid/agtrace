use crate::presentation::formatters::{text, time};
use agtrace_index::SessionSummary;
use owo_colors::OwoColorize;
use std::fmt;

pub struct SessionListView {
    sessions: Vec<SessionSummary>,
}

impl SessionListView {
    pub fn from_summaries(sessions: Vec<SessionSummary>) -> Self {
        Self { sessions }
    }
}

impl fmt::Display for SessionListView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for session in &self.sessions {
            let id_short = if session.id.len() > 8 {
                &session.id[..8]
            } else {
                &session.id
            };

            let time_str = session.start_ts.as_deref().unwrap_or("unknown");
            let time_display = time::format_relative_time(time_str);

            let snippet = session.snippet.as_deref().unwrap_or("");
            let snippet_display = text::normalize_and_clean(snippet, 80);

            let provider_display = match session.provider.as_str() {
                "claude_code" => format!("{}", session.provider.blue()),
                "codex" => format!("{}", session.provider.green()),
                "gemini" => format!("{}", session.provider.red()),
                _ => session.provider.clone(),
            };

            let snippet_final = if snippet_display.is_empty() {
                format!("{}", "[empty]".bright_black())
            } else {
                snippet_display
            };

            writeln!(
                f,
                "{} {} {} {}",
                time_display.bright_black(),
                id_short.yellow(),
                provider_display,
                snippet_final
            )?;
        }
        Ok(())
    }
}
