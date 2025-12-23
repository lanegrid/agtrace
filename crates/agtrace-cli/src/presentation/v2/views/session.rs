use std::fmt;

use crate::presentation::v2::formatters::{display, json, number, text, time};
use crate::presentation::v2::view_models::{
    AgentStepViewModel, SessionAnalysisViewModel, SessionListViewModel, ViewMode,
};

// --------------------------------------------------------
// Session List View
// --------------------------------------------------------

pub struct SessionListView<'a> {
    data: &'a SessionListViewModel,
    mode: ViewMode,
}

impl<'a> SessionListView<'a> {
    /// Create a new SessionListView (called from CreateView trait)
    pub fn new(data: &'a SessionListViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Minimal: Just IDs, one per line (for pipes/scripts)
        for session in &self.data.sessions {
            writeln!(f, "{}", session.id)?;
        }
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Compact: One line per session with key info
        if self.data.sessions.is_empty() {
            writeln!(f, "No sessions found.")?;
            return Ok(());
        }

        for session in &self.data.sessions {
            // Shorten ID to first 8 chars (like git commit hash)
            let id_short = session.id.chars().take(8).collect::<String>();

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
        if self.data.sessions.is_empty() {
            writeln!(f, "No sessions found.")?;
            self.show_filter_info(f)?;
            return Ok(());
        }

        for session in &self.data.sessions {
            let id_short = if session.id.len() > 8 {
                &session.id[..8]
            } else {
                &session.id
            };

            let time_str = session.start_ts.as_deref().unwrap_or("unknown");
            let snippet = session.snippet.as_deref().unwrap_or("[empty]");

            writeln!(
                f,
                "{} {} {} {}",
                time_str, id_short, session.provider, snippet
            )?;
        }

        self.show_filter_info(f)?;

        Ok(())
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Verbose: Table + all available metadata
        if self.data.sessions.is_empty() {
            writeln!(f, "No sessions found.")?;
            self.show_filter_info(f)?;
            return Ok(());
        }

        for session in &self.data.sessions {
            let id_short = if session.id.len() > 8 {
                &session.id[..8]
            } else {
                &session.id
            };

            let time_str = session.start_ts.as_deref().unwrap_or("unknown");
            let snippet = session.snippet.as_deref().unwrap_or("[empty]");

            writeln!(
                f,
                "{} {} {} {}",
                time_str, id_short, session.provider, snippet
            )?;
        }

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

impl<'a> fmt::Display for SessionListView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}

// --------------------------------------------------------
// Session Analysis View
// --------------------------------------------------------

pub struct SessionAnalysisView<'a> {
    data: &'a SessionAnalysisViewModel,
    mode: ViewMode,
}

impl<'a> SessionAnalysisView<'a> {
    /// Create a new SessionAnalysisView (called from CreateView trait)
    pub fn new(data: &'a SessionAnalysisViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
            let query_preview = text::truncate(&turn.user_query, 50);
            writeln!(
                f,
                "  #{:02} | {} tools | {}",
                turn.turn_number, tool_count, query_preview
            )?;
        }
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
            write!(f, "{}", TurnView::new(turn))?;
        }

        Ok(())
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
            write!(f, "{}", TurnView::new(turn))?;
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

// --------------------------------------------------------
// Turn View (extracted from TurnAnalysisViewModel Display)
// --------------------------------------------------------

use crate::presentation::v2::view_models::TurnAnalysisViewModel;

pub struct TurnView<'a> {
    data: &'a TurnAnalysisViewModel,
}

impl<'a> TurnView<'a> {
    pub fn new(data: &'a TurnAnalysisViewModel) -> Self {
        Self { data }
    }

    fn extract_delta_indicator(&self) -> String {
        let delta_value = self.data.metrics.total_delta;

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
        let truncated = text::truncate(result, 60);
        writeln!(f, "{}", truncated)?;
        Ok(())
    }
}

impl<'a> fmt::Display for TurnView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let status_icon = if self.data.is_heavy_load {
            "⚠️"
        } else {
            "🟢"
        };
        let status_label = if self.data.is_heavy_load {
            "Heavy Load"
        } else {
            "Normal"
        };

        // Extract delta for visual indicator
        let delta_indicator = self.extract_delta_indicator();

        // Build context transition display
        let prev_str = number::format_compact(self.data.prev_tokens as i64);
        let curr_str = number::format_compact(self.data.current_tokens as i64);
        let context_transition = format!("{} -> {}", prev_str, curr_str);

        writeln!(
            f,
            "[Turn #{:02}] {} {}  (Context: {}{})",
            self.data.turn_number, status_icon, status_label, context_transition, delta_indicator
        )?;

        // Show progress bar if context usage data is available
        if let Some(ref usage) = self.data.context_usage {
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
        let total_items = 1 + self.data.steps.len();
        let mut current_index = 0;

        // User query
        current_index += 1;
        let is_last = current_index == total_items;
        let prefix = if is_last { "└──" } else { "├──" };
        writeln!(f, "{} 👤 User", prefix)?;
        let truncated_query = text::truncate(&self.data.user_query, 80);
        self.write_indented(f, &truncated_query, is_last, "   ")?;

        // Steps
        for step in &self.data.steps {
            current_index += 1;
            let is_last = current_index == total_items;
            self.write_step(f, step, is_last)?;
        }

        writeln!(f)?;

        // Format metrics
        let delta_str = format!(
            "+{}",
            number::format_compact(self.data.metrics.total_delta as i64)
        );
        let input_str = number::format_compact(self.data.metrics.input_tokens);
        let output_str = number::format_compact(self.data.metrics.output_tokens);
        let cache_str = self
            .data
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

// --------------------------------------------------------
// Helper Functions
// --------------------------------------------------------

/// Format tool arguments for display
fn format_tool_args(tool_call: &agtrace_types::ToolCallPayload) -> String {
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
