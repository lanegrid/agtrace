use crate::reactor::SessionState;
use agtrace_types::AgentEvent;
use std::collections::VecDeque;
use std::sync::Mutex;

pub trait TerminalWriter: Send {
    fn clear_screen(&mut self);
    fn write_line(&mut self, line: &str);
    fn flush(&mut self);
}

pub struct WatchBuffer {
    events: VecDeque<AgentEvent>,
    max_events: usize,
    state: SessionState,
}

impl WatchBuffer {
    pub fn new(max_events: usize) -> Self {
        use chrono::Utc;
        Self {
            events: VecDeque::with_capacity(max_events),
            max_events,
            state: SessionState::new("test".to_string(), None, Utc::now()),
        }
    }

    pub fn push_event(&mut self, event: AgentEvent) {
        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    pub fn update_state(&mut self, state: SessionState) {
        self.state = state;
    }

    pub fn format_content(&self) -> Vec<String> {
        let mut lines = Vec::new();

        for event in &self.events {
            lines.push(self.format_event(event));
        }

        lines
    }

    pub fn format_footer(&self) -> Vec<String> {
        use crate::display_model::{DisplayOptions, TokenSummaryDisplay};
        use crate::token_limits::TokenLimits;
        use crate::views::session::format_token_summary;

        let token_limits = TokenLimits::new();
        let limit = self.state.context_window_limit.or_else(|| {
            self.state
                .model
                .as_ref()
                .and_then(|m| token_limits.get_limit(m).map(|l| l.total_limit))
        });

        let summary = TokenSummaryDisplay {
            input: self.state.total_input_side_tokens(),
            output: self.state.total_output_side_tokens(),
            cache_creation: self.state.current_usage.cache_creation.0,
            cache_read: self.state.current_usage.cache_read.0,
            total: self.state.total_context_window_tokens(),
            limit,
            model: self.state.model.clone(),
        };

        let opts = DisplayOptions {
            enable_color: true,
            relative_time: false,
            truncate_text: None,
        };

        format_token_summary(&summary, &opts)
    }

    fn format_event(&self, event: &AgentEvent) -> String {
        use agtrace_types::EventPayload;
        use chrono::Local;
        use owo_colors::OwoColorize;

        let time = event.timestamp.with_timezone(&Local).format("%H:%M:%S");

        match &event.payload {
            EventPayload::User(payload) => {
                let text = self.truncate(&payload.text, 100);
                format!("{} {} \"{}\"", time.dimmed(), "👤 User:".bold(), text)
            }
            EventPayload::Reasoning(payload) => {
                let text = self.truncate(&payload.text, 50);
                format!(
                    "{} {} {}",
                    time.dimmed(),
                    "🧠 Thnk:".dimmed(),
                    text.dimmed()
                )
            }
            EventPayload::Message(payload) => {
                let text = self.truncate(&payload.text, 100);
                format!("{} {} {}", time, "💬 Msg:".cyan(), text)
            }
            EventPayload::ToolCall(payload) => {
                format!("{} 🧪 {}", time, payload.name.magenta())
            }
            EventPayload::TokenUsage(_) => String::new(),
            _ => String::new(),
        }
    }

    fn truncate(&self, text: &str, max_len: usize) -> String {
        if text.chars().count() <= max_len {
            text.to_string()
        } else {
            let chars: Vec<char> = text.chars().take(max_len - 3).collect();
            format!("{}...", chars.iter().collect::<String>())
        }
    }
}

pub struct MockTerminal {
    pub lines: Vec<String>,
    pub clear_count: usize,
    pub flush_count: usize,
}

impl Default for MockTerminal {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTerminal {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            clear_count: 0,
            flush_count: 0,
        }
    }
}

impl TerminalWriter for MockTerminal {
    fn clear_screen(&mut self) {
        self.clear_count += 1;
        self.lines.clear();
    }

    fn write_line(&mut self, line: &str) {
        self.lines.push(line.to_string());
    }

    fn flush(&mut self) {
        self.flush_count += 1;
    }
}

pub struct AnsiTerminal;

impl Default for AnsiTerminal {
    fn default() -> Self {
        Self::new()
    }
}

impl AnsiTerminal {
    pub fn new() -> Self {
        Self
    }
}

impl TerminalWriter for AnsiTerminal {
    fn clear_screen(&mut self) {
        print!("\x1B[2J\x1B[1;1H");
    }

    fn write_line(&mut self, line: &str) {
        println!("{}", line);
    }

    fn flush(&mut self) {
        use std::io::{self, Write};
        let _ = io::stdout().flush();
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
    pub fn new(terminal: Box<dyn TerminalWriter>, max_events: usize) -> Self {
        Self {
            inner: Mutex::new(RefreshingWatchViewInner {
                buffer: WatchBuffer::new(max_events),
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

        let header = inner.header.clone();
        let content = inner.buffer.format_content();
        let footer = inner.buffer.format_footer();

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
}

impl crate::ui::traits::SystemView for RefreshingWatchView {
    fn render_guidance(&self, _context: &crate::ui::models::GuidanceContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_provider_list(
        &self,
        _providers: &[crate::ui::models::ProviderConfigSummary],
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_provider_detected(
        &self,
        _providers: &[crate::ui::models::ProviderConfigSummary],
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_provider_set(
        &self,
        _result: &crate::ui::models::ProviderSetResult,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_warning(&self, _message: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_project_list(
        &self,
        _current_root: &str,
        _current_hash: &str,
        _projects: &[crate::ui::models::ProjectSummary],
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_corpus_overview(
        &self,
        _stats: &crate::ui::models::CorpusStats,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_index_event(&self, _event: crate::ui::models::IndexEvent) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_init_event(&self, _event: crate::ui::models::InitRenderEvent) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_lab_export(
        &self,
        _exported: usize,
        _output_path: &std::path::Path,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl crate::ui::traits::SessionView for RefreshingWatchView {
    fn render_session_list(
        &self,
        _sessions: &[agtrace_index::SessionSummary],
        _format: crate::types::OutputFormat,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_session_raw_files(
        &self,
        _files: &[crate::ui::models::RawFileContent],
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_session_events_json(&self, _events: &[AgentEvent]) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_session_compact(
        &self,
        _display: &crate::display_model::SessionDisplay,
        _options: &crate::display_model::DisplayOptions,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_session_timeline(
        &self,
        _events: &[AgentEvent],
        _truncate: bool,
        _enable_color: bool,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_session_assemble_error(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_pack_report(
        &self,
        _digests: &[agtrace_engine::SessionDigest],
        _template: crate::ui::models::ReportTemplate,
        _pool_size: usize,
        _candidate_count: usize,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl crate::ui::traits::DiagnosticView for RefreshingWatchView {
    fn render_doctor_check(
        &self,
        _display: &crate::display_model::DoctorCheckDisplay,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_diagnose_results(
        &self,
        _results: &[agtrace_engine::DiagnoseResult],
        _verbose: bool,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_inspect(&self, _display: &crate::ui::models::InspectDisplay) -> anyhow::Result<()> {
        Ok(())
    }

    fn render_provider_schema(
        &self,
        _content: &crate::display_model::ProviderSchemaContent,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl crate::ui::traits::WatchView for RefreshingWatchView {
    fn render_watch_start(&self, start: &crate::ui::models::WatchStart) -> anyhow::Result<()> {
        use crate::ui::models::WatchStart;
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
        _summary: &crate::ui::models::WatchSummary,
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

    fn on_watch_reaction(&self, _reaction: &crate::reactor::Reaction) -> anyhow::Result<()> {
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
    use agtrace_types::{EventPayload, UserPayload};
    use chrono::Utc;

    fn create_user_event(text: &str) -> AgentEvent {
        use uuid::Uuid;
        AgentEvent {
            id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
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
        let mut buffer = WatchBuffer::new(5);
        buffer.push_event(create_user_event("test1"));
        buffer.push_event(create_user_event("test2"));

        let lines = buffer.format_content();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_buffer_max_events() {
        let mut buffer = WatchBuffer::new(3);
        for i in 0..5 {
            buffer.push_event(create_user_event(&format!("test{}", i)));
        }

        let lines = buffer.format_content();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("test2"));
        assert!(lines[2].contains("test4"));
    }

    #[test]
    fn test_buffer_formatting() {
        let mut buffer = WatchBuffer::new(10);
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
        let view = RefreshingWatchView::new(Box::new(MockTerminal::new()), 10);

        view.on_event(create_user_event("test message"));
        view.render();
    }

    #[test]
    fn test_footer_contains_context_window() {
        let buffer = WatchBuffer::new(10);
        let footer = buffer.format_footer();

        assert!(
            footer.is_empty()
                || footer
                    .iter()
                    .any(|l| l.contains("Context Window") || l.contains("⛁") || l.contains("⛶"))
        );
    }
}
