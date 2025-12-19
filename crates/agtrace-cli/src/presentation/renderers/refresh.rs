use crate::presentation::formatters::{text, time, tool};
use crate::presentation::renderers::backend::TerminalWriter;
use agtrace_runtime::reactor::SessionState;
use agtrace_types::AgentEvent;
use std::collections::VecDeque;
use std::sync::Mutex;

pub struct WatchBuffer {
    events: VecDeque<AgentEvent>,
    state: SessionState,
}

impl Default for WatchBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl WatchBuffer {
    pub fn new() -> Self {
        use chrono::Utc;
        Self {
            events: VecDeque::new(),
            state: SessionState::new("test".to_string(), None, Utc::now()),
        }
    }

    pub fn push_event(&mut self, event: AgentEvent) {
        self.events.push_back(event);
    }

    pub fn update_state(&mut self, state: SessionState) {
        self.state = state;
    }

    pub fn format_header(&self) -> Vec<String> {
        use owo_colors::OwoColorize;
        let mut lines = Vec::new();

        // Project info
        if let Some(root) = &self.state.project_root {
            let root_str = root.display().to_string();
            lines.push(format!("{} {}", "📁 Project:".bold(), root_str.cyan()));

            // Calculate project hash
            let hash = agtrace_types::project_hash_from_root(&root_str);
            let short_hash = &hash[..8];
            lines.push(format!("{} {}", "🔖 Hash:".bold(), short_hash.dimmed()));
        }

        lines
    }

    pub fn format_content(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let mut prev_time: Option<chrono::DateTime<chrono::Utc>> = None;

        for event in &self.events {
            let delta = if let Some(prev) = prev_time {
                let diff = event.timestamp.signed_duration_since(prev);
                time::format_delta_time(diff)
            } else {
                None
            };
            lines.push(self.format_event(event, delta));
            prev_time = Some(event.timestamp);
        }

        lines
    }

    pub fn format_footer(&self) -> Vec<String> {
        use crate::presentation::formatters::token::TokenUsageView;
        use crate::presentation::formatters::DisplayOptions;

        let opts = DisplayOptions {
            enable_color: true,
            relative_time: false,
            truncate_text: None,
        };

        let token_view = TokenUsageView::from_state(&self.state, opts);
        format!("{}", token_view)
            .lines()
            .map(|s| s.to_string())
            .collect()
    }

    fn format_event(&self, event: &AgentEvent, delta: Option<String>) -> String {
        use agtrace_types::{EventPayload, StreamId};
        use chrono::Local;
        use owo_colors::OwoColorize;

        let time = event.timestamp.with_timezone(&Local).format("%H:%M:%S");
        let delta_str = delta
            .map(|d| format!(" {}", d.dimmed()))
            .unwrap_or_default();

        // Check if this is a sidechain event
        if let StreamId::Sidechain { agent_id } = &event.stream_id {
            let short_id = &agent_id.chars().take(6).collect::<String>();
            return format!(
                "{}{} {} Sidechain #{} spawned",
                time.dimmed(),
                delta_str,
                "⛓️".yellow(),
                short_id.yellow()
            );
        }

        match &event.payload {
            EventPayload::User(payload) => {
                let txt = text::truncate(&payload.text, 100);
                format!(
                    "{}{} {} \"{}\"",
                    time.dimmed(),
                    delta_str,
                    "👤 User:".bold(),
                    txt
                )
            }
            EventPayload::Reasoning(payload) => {
                let txt = text::truncate(&payload.text, 50);
                format!(
                    "{}{} {} {}",
                    time.dimmed(),
                    delta_str,
                    "🧠 Thnk:".dimmed(),
                    txt.cyan()
                )
            }
            EventPayload::Message(payload) => {
                let txt = text::truncate(&payload.text, 100);
                format!(
                    "{}{} {} {}",
                    time.dimmed(),
                    delta_str,
                    "💬 Msg:".cyan(),
                    txt
                )
            }
            EventPayload::ToolCall(payload) => {
                let (icon, color_fn) = tool::categorize_tool(&payload.name);
                let summary = tool::format_tool_call(
                    &payload.name,
                    &payload.arguments,
                    self.state.project_root.as_deref(),
                );
                let colored_name = color_fn(&payload.name);
                format!(
                    "{}{} {} {}: {}",
                    time.dimmed(),
                    delta_str,
                    icon,
                    colored_name,
                    summary
                )
            }
            EventPayload::ToolResult(payload) => {
                if payload.is_error {
                    let output = text::truncate(&payload.output, 100);
                    format!(
                        "{}{} {} {}",
                        time.dimmed(),
                        delta_str,
                        "❌ Fail:".red(),
                        output.red()
                    )
                } else {
                    String::new()
                }
            }
            EventPayload::TokenUsage(_) => String::new(),
            _ => String::new(),
        }
    }
}

struct RefreshingWatchViewInner {
    buffer: WatchBuffer,
    terminal: Box<dyn TerminalWriter>,
    header: Vec<String>,
}

pub struct RefreshingWatchView {
    inner: Mutex<RefreshingWatchViewInner>,
}

impl RefreshingWatchView {
    pub fn new(terminal: Box<dyn TerminalWriter>) -> Self {
        Self {
            inner: Mutex::new(RefreshingWatchViewInner {
                buffer: WatchBuffer::new(),
                terminal,
                header: Vec::new(),
            }),
        }
    }

    fn on_event(&self, event: AgentEvent) {
        self.inner.lock().unwrap().buffer.push_event(event);
    }

    fn on_state_update(&self, state: SessionState) {
        self.inner.lock().unwrap().buffer.update_state(state);
    }

    fn render(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.terminal.clear_screen();

        // Combine watch header (session info) and project info
        let mut header = inner.header.clone();
        header.extend(inner.buffer.format_header());
        let mut content = inner.buffer.format_content();
        let footer = inner.buffer.format_footer();

        // Calculate available lines for content
        let terminal_height = self.get_terminal_height();
        let header_lines = if header.is_empty() {
            0
        } else {
            header.len() + 1
        };
        let footer_lines = if footer.is_empty() {
            0
        } else {
            footer.len() + 1
        };
        let available_lines = terminal_height.saturating_sub(header_lines + footer_lines);

        // Trim content to fit terminal
        if content.len() > available_lines {
            let skip = content.len() - available_lines;
            content = content.into_iter().skip(skip).collect();
        }

        for line in &header {
            inner.terminal.write_line(line);
        }

        if !header.is_empty() {
            inner.terminal.write_line("");
        }

        for line in content {
            if !line.is_empty() {
                inner.terminal.write_line(&line);
            }
        }

        if !footer.is_empty() {
            inner.terminal.write_line("");
            for line in footer {
                inner.terminal.write_line(&line);
            }
        }

        inner.terminal.flush();
    }

    fn get_terminal_height(&self) -> usize {
        if let Some((_, terminal_size::Height(h))) = terminal_size::terminal_size() {
            h as usize
        } else {
            24 // Default fallback
        }
    }
}

impl super::traits::WatchView for RefreshingWatchView {
    fn render_watch_start(&self, start: &super::models::WatchStart) -> anyhow::Result<()> {
        use super::models::WatchStart;
        use owo_colors::OwoColorize;

        let header = match start {
            WatchStart::Provider { name, log_root } => {
                format!(
                    "{} {} ({})",
                    "[👀 Watching]".bright_cyan(),
                    log_root.display(),
                    name
                )
            }
            WatchStart::Session { id, log_root } => {
                format!(
                    "{} session {} in {}",
                    "[👀 Watching]".bright_cyan(),
                    id,
                    log_root.display()
                )
            }
        };

        self.inner.lock().unwrap().header.push(header);
        Ok(())
    }

    fn on_watch_attached(&self, display_name: &str) -> anyhow::Result<()> {
        use owo_colors::OwoColorize;
        let msg = format!(
            "{}  {}",
            "✨ Attached to active session:".bright_green(),
            display_name
        );
        self.inner.lock().unwrap().header.push(msg);
        Ok(())
    }

    fn on_watch_initial_summary(
        &self,
        _summary: &super::models::WatchSummary,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_watch_rotated(
        &self,
        old_path: &std::path::Path,
        new_path: &std::path::Path,
    ) -> anyhow::Result<()> {
        use owo_colors::OwoColorize;
        let old_name = old_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| old_path.display().to_string());
        let new_name = new_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| new_path.display().to_string());

        let mut inner = self.inner.lock().unwrap();
        inner.header.clear();
        inner.header.push(format!(
            "{} {} → {}",
            "✨ Session rotated:".bright_green(),
            old_name.dimmed(),
            new_name
        ));
        Ok(())
    }

    fn on_watch_waiting(&self, message: &str) -> anyhow::Result<()> {
        use owo_colors::OwoColorize;
        self.inner.lock().unwrap().header.push(format!(
            "{} {}",
            "[⏳ Waiting]".bright_yellow(),
            message
        ));
        Ok(())
    }

    fn on_watch_error(&self, message: &str, _fatal: bool) -> anyhow::Result<()> {
        use owo_colors::OwoColorize;
        self.inner
            .lock()
            .unwrap()
            .header
            .push(format!("{} {}", "❌ Error:".red(), message));
        Ok(())
    }

    fn on_watch_orphaned(&self, _orphaned: usize, _total_events: usize) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_watch_token_warning(&self, _warning: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_watch_reactor_error(&self, _reactor_name: &str, _error: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_watch_reaction_error(&self, _error: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_watch_reaction(
        &self,
        _reaction: &agtrace_runtime::reactor::Reaction,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_stream_update(
        &self,
        state: &SessionState,
        new_events: &[AgentEvent],
    ) -> anyhow::Result<()> {
        for event in new_events {
            self.on_event(event.clone());
        }

        self.on_state_update(state.clone());
        self.render();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::renderers::backend::MockTerminal;
    use agtrace_types::{EventPayload, UserPayload};
    use chrono::Utc;

    fn create_user_event(text: &str) -> AgentEvent {
        use uuid::Uuid;
        AgentEvent {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            parent_id: None,
            timestamp: Utc::now(),
            stream_id: Default::default(),
            payload: EventPayload::User(UserPayload {
                text: text.to_string(),
            }),
            metadata: None,
        }
    }

    #[test]
    fn test_buffer_push_event() {
        let mut buffer = WatchBuffer::new();
        buffer.push_event(create_user_event("test1"));
        buffer.push_event(create_user_event("test2"));

        let lines = buffer.format_content();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_buffer_formatting() {
        let mut buffer = WatchBuffer::new();
        buffer.push_event(create_user_event("hello world"));

        let lines = buffer.format_content();
        assert!(lines[0].contains("User:"));
        assert!(lines[0].contains("hello world"));
    }

    #[test]
    fn test_mock_terminal() {
        let mut terminal = MockTerminal::new();
        terminal.clear_screen();
        terminal.write_line("line1");
        terminal.write_line("line2");
        terminal.flush();

        assert_eq!(terminal.clear_count, 1);
        assert_eq!(terminal.lines.len(), 2);
        assert_eq!(terminal.flush_count, 1);
    }

    #[test]
    fn test_refreshing_view_render() {
        let view = RefreshingWatchView::new(Box::new(MockTerminal::new()));

        view.on_event(create_user_event("test message"));
        view.render();
    }

    #[test]
    fn test_footer_contains_context_window() {
        let buffer = WatchBuffer::new();
        let footer = buffer.format_footer();

        assert!(
            footer.is_empty()
                || footer
                    .iter()
                    .any(|l| l.contains("Context Window") || l.contains("⛁") || l.contains("⛶"))
        );
    }
}
