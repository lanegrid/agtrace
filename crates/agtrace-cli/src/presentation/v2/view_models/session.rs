use serde::Serialize;
use std::fmt;

use super::{CreateView, ViewMode};

#[derive(Debug, Serialize)]
pub struct SessionListViewModel {
    pub sessions: Vec<SessionListEntry>,
    pub total_count: usize,
    pub applied_filters: FilterSummary,
}

#[derive(Debug, Serialize)]
pub struct SessionListEntry {
    pub id: String,
    pub provider: String,
    pub project_hash: String,
    pub start_ts: Option<String>,
    pub snippet: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FilterSummary {
    pub project_filter: Option<String>,
    pub source_filter: Option<String>,
    pub time_range: Option<String>,
    pub limit: usize,
}

impl CreateView for SessionListViewModel {
    fn create_view<'a>(&'a self, mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(SessionListView2 { data: self, mode })
    }
}

struct SessionListView2<'a> {
    data: &'a SessionListViewModel,
    mode: ViewMode,
}

impl<'a> SessionListView2<'a> {
    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Minimal: Just IDs, one per line (for pipes/scripts)
        for session in &self.data.sessions {
            writeln!(f, "{}", session.id)?;
        }
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Compact: One line per session with key info
        use crate::presentation::formatters::{text, time};

        if self.data.sessions.is_empty() {
            writeln!(f, "No sessions found.")?;
            return Ok(());
        }

        for session in &self.data.sessions {
            // Shorten ID to first 8 chars (like git commit hash)
            let id_short = if session.id.len() > 8 {
                &session.id[..8]
            } else {
                &session.id
            };

            // Use relative time format
            let time_display = session
                .start_ts
                .as_ref()
                .map(|ts| time::format_relative_time(ts))
                .unwrap_or_else(|| "(no timestamp)".to_string());

            // Inline snippet by replacing newlines with spaces
            let snippet = session
                .snippet
                .as_ref()
                .map(|s| text::normalize_and_clean(s, 50))
                .unwrap_or_else(|| "(no snippet)".to_string());

            writeln!(
                f,
                "{} {} [{}] {}",
                id_short, time_display, session.provider, snippet
            )?;
        }
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Standard: Table format (current behavior)
        use crate::presentation::formatters::session_list::SessionEntry;
        use crate::presentation::formatters::SessionListView;

        if self.data.sessions.is_empty() {
            writeln!(f, "No sessions found.")?;
            self.show_filter_info(f)?;
            return Ok(());
        }

        let entries: Vec<SessionEntry> = self
            .data
            .sessions
            .iter()
            .map(|s| SessionEntry {
                id: s.id.clone(),
                provider: s.provider.clone(),
                start_ts: s.start_ts.clone(),
                snippet: s.snippet.clone(),
            })
            .collect();

        write!(f, "{}", SessionListView::from_entries(entries))?;
        self.show_filter_info(f)?;

        Ok(())
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Verbose: Table + all available metadata
        use crate::presentation::formatters::session_list::SessionEntry;
        use crate::presentation::formatters::SessionListView;

        if self.data.sessions.is_empty() {
            writeln!(f, "No sessions found.")?;
            self.show_filter_info(f)?;
            return Ok(());
        }

        let entries: Vec<SessionEntry> = self
            .data
            .sessions
            .iter()
            .map(|s| SessionEntry {
                id: s.id.clone(),
                provider: s.provider.clone(),
                start_ts: s.start_ts.clone(),
                snippet: s.snippet.clone(),
            })
            .collect();

        write!(f, "{}", SessionListView::from_entries(entries))?;

        // In verbose mode, show project hashes for each session
        writeln!(f)?;
        writeln!(f, "Project information:")?;
        writeln!(f, "{:<50} PROJECT_HASH", "SESSION_ID")?;
        writeln!(f, "{}", "-".repeat(80))?;
        for session in &self.data.sessions {
            writeln!(f, "{:<50} {}", session.id, session.project_hash)?;
        }

        self.show_filter_info(f)?;

        Ok(())
    }

    fn show_filter_info(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.data.applied_filters.project_filter.is_some()
            || self.data.applied_filters.source_filter.is_some()
            || self.data.applied_filters.time_range.is_some()
        {
            writeln!(f)?;
            writeln!(f, "Filters applied:")?;
            if let Some(ref project) = self.data.applied_filters.project_filter {
                writeln!(f, "  Project: {}", project)?;
            }
            if let Some(ref source) = self.data.applied_filters.source_filter {
                writeln!(f, "  Source: {}", source)?;
            }
            if let Some(ref range) = self.data.applied_filters.time_range {
                writeln!(f, "  Time range: {}", range)?;
            }
        }
        Ok(())
    }
}

impl<'a> fmt::Display for SessionListView2<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}

impl fmt::Display for SessionListViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            SessionListView2 {
                data: self,
                mode: ViewMode::default(),
            }
        )
    }
}

/// Session analysis view - TUI-centric performance report
#[derive(Debug, Serialize)]
pub struct SessionAnalysisViewModel {
    pub header: SessionHeader,
    pub context_summary: ContextWindowSummary,
    pub turns: Vec<TurnAnalysisViewModel>,
}

#[derive(Debug, Serialize)]
pub struct SessionHeader {
    pub session_id: String,
    pub provider: String,
    pub model: Option<String>,
    pub status: String,
    pub duration: Option<String>,
    pub start_time: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ContextWindowSummary {
    pub current_tokens: u32,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct TurnAnalysisViewModel {
    pub turn_number: usize,
    pub timestamp: Option<String>,
    pub prev_tokens: u32,
    pub current_tokens: u32,
    pub context_usage: Option<ContextUsage>,
    pub is_heavy_load: bool,
    pub user_query: String,
    pub steps: Vec<AgentStepViewModel>,
    pub metrics: TurnMetrics,
}

#[derive(Debug, Serialize)]
pub struct ContextUsage {
    pub current_tokens: u32,
    pub max_tokens: u32,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub enum AgentStepViewModel {
    Thinking {
        duration: Option<String>,
        preview: String,
    },
    ToolCall {
        name: String,
        #[serde(skip)]
        arguments: agtrace_types::ToolCallPayload,
        #[serde(rename = "args")]
        args_formatted: Option<String>, // For JSON serialization compatibility
        result: String,
        is_error: bool,
    },
    ToolCallSequence {
        name: String,
        count: usize,
        #[serde(skip)]
        sample_arguments: agtrace_types::ToolCallPayload,
        #[serde(rename = "sample_args")]
        sample_args_formatted: Option<String>, // For JSON serialization compatibility
        has_errors: bool,
    },
    Message {
        text: String,
    },
    SystemEvent {
        description: String,
    },
}

#[derive(Debug, Serialize)]
pub struct TurnMetrics {
    pub total_delta: u32,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: Option<i64>,
}

impl CreateView for SessionAnalysisViewModel {
    fn create_view<'a>(&'a self, mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(SessionAnalysisView { data: self, mode })
    }
}

// Wrapper struct to separate View from ViewModel
struct SessionAnalysisView<'a> {
    data: &'a SessionAnalysisViewModel,
    mode: ViewMode,
}

impl<'a> SessionAnalysisView<'a> {
    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::presentation::v2::formatters::number;

        // Minimal: Just session ID and turn count
        writeln!(f, "{}", self.data.header.session_id)?;
        writeln!(
            f,
            "Turns: {} | Tokens: {}",
            self.data.turns.len(),
            number::format_compact(self.data.context_summary.current_tokens as i64)
        )?;
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::presentation::v2::formatters::number;

        // Compact: Single-line header + one-line per turn
        writeln!(
            f,
            "{} | {} | {} turns | {} tokens",
            self.data.header.session_id,
            self.data.header.provider,
            self.data.turns.len(),
            number::format_compact(self.data.context_summary.current_tokens as i64)
        )?;

        for turn in &self.data.turns {
            let tool_count = turn
                .steps
                .iter()
                .filter(|s| matches!(s, AgentStepViewModel::ToolCall { .. }))
                .count();
            let query_preview = if turn.user_query.len() > 50 {
                format!("{}...", &turn.user_query[..47])
            } else {
                turn.user_query.clone()
            };
            writeln!(
                f,
                "  #{:02} | {} tools | {}",
                turn.turn_number, tool_count, query_preview
            )?;
        }
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::presentation::v2::formatters::{display, number};

        // Standard: Full display (current behavior)
        writeln!(f, "{}", "=".repeat(80))?;
        write!(f, "SESSION: {}", self.data.header.session_id)?;
        if let Some(ref model) = self.data.header.model {
            write!(f, " ({})", model)?;
        }
        writeln!(f)?;
        writeln!(f, "STATUS:  {}", self.data.header.status)?;

        // Context summary
        let context_display = if let Some(max) = self.data.context_summary.max_tokens {
            let bar =
                display::build_progress_bar(self.data.context_summary.current_tokens, max, 40);
            format!(
                "{} ({} / {})",
                bar,
                number::format_compact(self.data.context_summary.current_tokens as i64),
                number::format_compact(max as i64)
            )
        } else {
            format!(
                "Total: {}",
                number::format_compact(self.data.context_summary.current_tokens as i64)
            )
        };
        writeln!(f, "CONTEXT: {}", context_display)?;

        writeln!(f, "{}", "=".repeat(80))?;
        writeln!(f)?;

        // Turns
        for turn in &self.data.turns {
            write!(f, "{}", turn)?;
        }

        Ok(())
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::presentation::v2::formatters::{display, number};

        // Verbose: Standard + additional metadata
        writeln!(f, "{}", "=".repeat(80))?;
        write!(f, "SESSION: {}", self.data.header.session_id)?;
        if let Some(ref model) = self.data.header.model {
            write!(f, " ({})", model)?;
        }
        writeln!(f)?;
        writeln!(f, "PROVIDER: {}", self.data.header.provider)?;
        writeln!(f, "STATUS:   {}", self.data.header.status)?;
        if let Some(ref start) = self.data.header.start_time {
            writeln!(f, "START:    {}", start)?;
        }
        if let Some(ref dur) = self.data.header.duration {
            writeln!(f, "DURATION: {}", dur)?;
        }

        // Context summary
        let context_display = if let Some(max) = self.data.context_summary.max_tokens {
            let bar =
                display::build_progress_bar(self.data.context_summary.current_tokens, max, 40);
            format!(
                "{} ({} / {})",
                bar,
                number::format_compact(self.data.context_summary.current_tokens as i64),
                number::format_compact(max as i64)
            )
        } else {
            format!(
                "Total: {}",
                number::format_compact(self.data.context_summary.current_tokens as i64)
            )
        };
        writeln!(f, "CONTEXT:  {}", context_display)?;

        writeln!(f, "{}", "=".repeat(80))?;
        writeln!(f)?;

        // Turns (verbose mode shows all details, same as standard)
        for turn in &self.data.turns {
            write!(f, "{}", turn)?;
        }

        Ok(())
    }
}

impl<'a> fmt::Display for SessionAnalysisView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}

impl fmt::Display for SessionAnalysisViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            SessionAnalysisView {
                data: self,
                mode: ViewMode::default(),
            }
        )
    }
}

impl fmt::Display for TurnAnalysisViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::presentation::v2::formatters::{display, number, text};

        let status_icon = if self.is_heavy_load { "⚠️" } else { "🟢" };
        let status_label = if self.is_heavy_load {
            "Heavy Load"
        } else {
            "Normal"
        };

        // Extract delta for visual indicator
        let delta_indicator = self.extract_delta_indicator();

        // Build context transition display
        let prev_str = number::format_compact(self.prev_tokens as i64);
        let curr_str = number::format_compact(self.current_tokens as i64);
        let context_transition = format!("{} -> {}", prev_str, curr_str);

        writeln!(
            f,
            "[Turn #{:02}] {} {}  (Context: {}{})",
            self.turn_number, status_icon, status_label, context_transition, delta_indicator
        )?;

        // Show progress bar if context usage data is available
        if let Some(ref usage) = self.context_usage {
            let bar = display::build_progress_bar(usage.current_tokens, usage.max_tokens, 20);
            writeln!(
                f,
                "│ {} ({} / {})",
                bar,
                number::format_compact(usage.current_tokens as i64),
                number::format_compact(usage.max_tokens as i64)
            )?;
        }
        writeln!(f, "│")?;

        // Calculate total items to display (user + steps)
        let total_items = 1 + self.steps.len();
        let mut current_index = 0;

        // User query
        current_index += 1;
        let is_last = current_index == total_items;
        let prefix = if is_last { "└──" } else { "├──" };
        writeln!(f, "{} 👤 User", prefix)?;
        let truncated_query = text::truncate(&self.user_query, 80);
        self.write_indented(f, &truncated_query, is_last, "   ")?;

        // Steps
        for step in &self.steps {
            current_index += 1;
            let is_last = current_index == total_items;
            self.write_step(f, step, is_last)?;
        }

        writeln!(f)?;

        // Format metrics
        let delta_str = format!(
            "+{}",
            number::format_compact(self.metrics.total_delta as i64)
        );
        let input_str = number::format_compact(self.metrics.input_tokens);
        let output_str = number::format_compact(self.metrics.output_tokens);
        let cache_str = self
            .metrics
            .cache_read_tokens
            .map(|c| format!(", Cache: {}", number::format_compact(c)))
            .unwrap_or_default();

        writeln!(
            f,
            "📊 Stats: {} (In: {}, Out: {}{})",
            delta_str, input_str, output_str, cache_str
        )?;
        writeln!(f)?;

        Ok(())
    }
}

impl TurnAnalysisViewModel {
    fn extract_delta_indicator(&self) -> String {
        // Use numeric value directly
        let delta_value = self.metrics.total_delta;

        if delta_value > 50_000 {
            " 🔺".to_string()
        } else if delta_value > 20_000 {
            " ⚡".to_string()
        } else {
            String::new()
        }
    }

    fn write_step(
        &self,
        f: &mut fmt::Formatter,
        step: &AgentStepViewModel,
        is_last: bool,
    ) -> fmt::Result {
        use crate::presentation::v2::formatters::text;

        let prefix = if is_last { "└──" } else { "├──" };
        let continuation = if is_last { "   " } else { "│  " };

        match step {
            AgentStepViewModel::Thinking { duration, preview } => {
                if let Some(dur) = duration {
                    writeln!(f, "{} 🧠 Thinking ({})", prefix, dur)?;
                } else {
                    writeln!(f, "{} 🧠 Thinking", prefix)?;
                }
                if !preview.is_empty() {
                    self.write_thinking_content(f, preview, is_last)?;
                }
            }
            AgentStepViewModel::ToolCall {
                name,
                arguments,
                result,
                is_error,
                ..
            } => {
                writeln!(f, "{} 🔧 Tool: {}", prefix, name)?;

                // Format arguments
                let args_str = format_tool_args(arguments);
                self.write_indented(f, &args_str, is_last, "   ")?;

                if *is_error {
                    write!(f, "{}   ↳ ❌ Error: ", continuation)?;
                } else {
                    write!(f, "{}   ↳ 📝 Result: ", continuation)?;
                }
                self.write_truncated_result(f, result)?;
            }
            AgentStepViewModel::ToolCallSequence {
                name,
                count,
                sample_arguments,
                has_errors,
                ..
            } => {
                let status = if *has_errors { "⚠️" } else { "✓" };
                writeln!(
                    f,
                    "{} 🔧 Tool: {} (x{} calls) {}",
                    prefix, name, count, status
                )?;

                // Format sample arguments
                let sample_args_str = format_tool_args(sample_arguments);
                self.write_indented(f, &sample_args_str, is_last, "   ")?;
            }
            AgentStepViewModel::Message { text } => {
                writeln!(f, "{} 💬 Message", prefix)?;
                let truncated = text::truncate(text, 80);
                self.write_indented(f, &truncated, is_last, "   ")?;
            }
            AgentStepViewModel::SystemEvent { description } => {
                writeln!(f, "{} ℹ️  {}", prefix, description)?;
            }
        }

        if !is_last {
            writeln!(f, "│")?;
        }

        Ok(())
    }

    fn write_thinking_content(
        &self,
        f: &mut fmt::Formatter,
        preview: &str,
        is_last: bool,
    ) -> fmt::Result {
        use crate::presentation::v2::formatters::text;

        let continuation = if is_last { "   " } else { "│  " };

        // Truncate and show first line prominently
        let truncated = text::truncate(preview, 60);
        let lines: Vec<&str> = truncated.lines().collect();
        if let Some(first_line) = lines.first() {
            writeln!(f, "{}   {}", continuation, first_line)?;
        }

        Ok(())
    }

    fn write_indented(
        &self,
        f: &mut fmt::Formatter,
        text: &str,
        is_last: bool,
        extra_indent: &str,
    ) -> fmt::Result {
        let continuation = if is_last { "   " } else { "│  " };

        for line in text.lines() {
            writeln!(f, "{}{}{}", continuation, extra_indent, line)?;
        }

        Ok(())
    }

    fn write_truncated_result(&self, f: &mut fmt::Formatter, result: &str) -> fmt::Result {
        use crate::presentation::v2::formatters::text;

        let truncated = text::truncate(result, 60);
        writeln!(f, "{}", truncated)?;
        Ok(())
    }
}

// Helper function to format tool arguments
fn format_tool_args(tool_call: &agtrace_types::ToolCallPayload) -> String {
    use crate::presentation::v2::formatters::json;
    use agtrace_types::ToolCallPayload;

    match tool_call {
        ToolCallPayload::FileRead { arguments, .. } => {
            json::format_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
        ToolCallPayload::FileEdit { arguments, .. } => {
            json::format_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
        ToolCallPayload::FileWrite { arguments, .. } => {
            json::format_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
        ToolCallPayload::Execute { arguments, .. } => {
            json::format_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
        ToolCallPayload::Search { arguments, .. } => {
            json::format_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
        ToolCallPayload::Mcp { arguments, .. } => {
            json::format_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
        ToolCallPayload::Generic { arguments, .. } => {
            json::format_compact(&serde_json::to_value(arguments).unwrap_or_default())
        }
    }
}
