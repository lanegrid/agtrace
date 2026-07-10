use std::fmt;

use crate::presentation::formatters::{display, json, number, text, time};
use crate::presentation::view_models::{
    AgentStepViewModel, SessionDetailViewModel, SessionListViewModel, SpawnContextViewModel,
    StreamAnalysisViewModel, ViewMode,
};

// Display constants
const SESSION_ID_SHORT_LENGTH: usize = 8;
const SNIPPET_MAX_LENGTH: usize = 50;
const QUERY_DISPLAY_LENGTH: usize = 200;
const QUERY_MAX_LINES: usize = 5;
const THINKING_PREVIEW_LENGTH: usize = 200;
const THINKING_MAX_LINES: usize = 5;
const MESSAGE_PREVIEW_LENGTH: usize = 200;
const MESSAGE_MAX_LINES: usize = 5;
const TOOL_RESULT_LENGTH: usize = 150;
const CONTEXT_BAR_WIDTH_STANDARD: usize = 40;
const CONTEXT_BAR_WIDTH_COMPACT: usize = 20;
const DELTA_WARNING_THRESHOLD: u32 = 20_000;
const DELTA_ALERT_THRESHOLD: u32 = 50_000;

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

        // Print header
        writeln!(
            f,
            "{:<8}  {:<12}  {:<11}  {:<35}  SNIPPET",
            "SESSION", "TIME", "PROVIDER", "PROJECT"
        )?;
        writeln!(f, "{}", "-".repeat(120))?;

        for session in &self.data.sessions {
            // Shorten ID to first 8 chars (like git commit hash)
            let id_short = session
                .id
                .chars()
                .take(SESSION_ID_SHORT_LENGTH)
                .collect::<String>();

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
                .map(|s| text::normalize_and_clean(s, SNIPPET_MAX_LENGTH))
                .unwrap_or_else(|| "--".to_string());

            // Shorten project root for display
            let project_display = if let Some(ref root) = session.project_root {
                text::shorten_home_path(root)
            } else {
                let hash_prefix = if session.project_hash.len() >= 8 {
                    &session.project_hash[..8]
                } else {
                    &session.project_hash
                };
                format!("{}...", hash_prefix)
            };

            writeln!(
                f,
                "{:<8}  {:<12}  {:<11}  {:<35}  {}",
                id_short, time_display, session.provider, project_display, snippet
            )?;
        }
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Standard: Table format with header and aligned columns
        if self.data.sessions.is_empty() {
            writeln!(f, "No sessions found.")?;
            self.show_filter_info(f)?;
            return Ok(());
        }

        // Print header
        writeln!(
            f,
            "{:<8}  {:<20}  {:<11}  {:<35}  SNIPPET",
            "SESSION", "TIME", "PROVIDER", "PROJECT"
        )?;
        writeln!(f, "{}", "-".repeat(120))?;

        // Print sessions
        for session in &self.data.sessions {
            let id_short = if session.id.len() > SESSION_ID_SHORT_LENGTH {
                &session.id[..SESSION_ID_SHORT_LENGTH]
            } else {
                &session.id
            };

            let time_str = session.start_ts.as_deref().unwrap_or("unknown");

            // Normalize snippet to single line
            let snippet = session
                .snippet
                .as_ref()
                .map(|s| text::normalize_and_clean(s, SNIPPET_MAX_LENGTH))
                .unwrap_or_else(|| "--".to_string());

            // Shorten project root for display
            let project_display = if let Some(ref root) = session.project_root {
                text::shorten_home_path(root)
            } else {
                let hash_prefix = if session.project_hash.len() >= 8 {
                    &session.project_hash[..8]
                } else {
                    &session.project_hash
                };
                format!("{}...", hash_prefix)
            };

            writeln!(
                f,
                "{:<8}  {:<20}  {:<11}  {:<35}  {}",
                id_short, time_str, session.provider, project_display, snippet
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
            let id_short = if session.id.len() > SESSION_ID_SHORT_LENGTH {
                &session.id[..SESSION_ID_SHORT_LENGTH]
            } else {
                &session.id
            };

            let time_str = session.start_ts.as_deref().unwrap_or("unknown");
            let snippet = session.snippet.as_deref().unwrap_or("--");

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
            || self.data.applied_filters.provider_filter.is_some()
            || self.data.applied_filters.time_range.is_some()
        {
            writeln!(f)?;
            writeln!(f, "Filters applied:")?;
            if let Some(ref project) = self.data.applied_filters.project_filter {
                writeln!(f, "  Project: {}", project)?;
            }
            if let Some(ref provider) = self.data.applied_filters.provider_filter {
                writeln!(f, "  Provider: {}", provider)?;
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
// Session Detail View
// --------------------------------------------------------

pub struct SessionDetailView<'a> {
    data: &'a SessionDetailViewModel,
    mode: ViewMode,
}

/// Format a spawn context for display (1-based indices).
fn format_spawn_context(ctx: &SpawnContextViewModel) -> String {
    format!(
        "spawned by Turn #{}, Step #{}",
        ctx.turn_index + 1,
        ctx.step_index + 1
    )
}

impl<'a> SessionDetailView<'a> {
    /// Create a new SessionDetailView (called from CreateView trait)
    pub fn new(data: &'a SessionDetailViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn main_stream(&self) -> Option<&StreamAnalysisViewModel> {
        self.data.streams.first()
    }

    fn extra_streams(&self) -> &[StreamAnalysisViewModel] {
        self.data.streams.get(1..).unwrap_or(&[])
    }

    /// Separator + heading for streams after the first (sidechains).
    fn write_stream_heading(
        &self,
        f: &mut fmt::Formatter,
        stream: &StreamAnalysisViewModel,
    ) -> fmt::Result {
        writeln!(f, "\n{}", "─".repeat(80))?;
        let spawn_info = stream
            .spawned_by
            .as_ref()
            .map(|ctx| format!(" ({})", format_spawn_context(ctx)))
            .unwrap_or_default();
        writeln!(f, "Stream: {}{}", stream.stream_id, spawn_info)?;
        writeln!(
            f,
            "Turns: {} | Tokens: {}\n",
            stream.turns.len(),
            number::format_compact(stream.context_summary.current_tokens as i64)
        )?;
        Ok(())
    }

    fn write_stream_turns(
        &self,
        f: &mut fmt::Formatter,
        stream: &StreamAnalysisViewModel,
    ) -> fmt::Result {
        for turn in &stream.turns {
            write!(f, "{}", TurnView::new(turn, self.mode))?;
        }
        Ok(())
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Minimal: Session ID, then one line per stream
        writeln!(f, "{}", self.data.session.session_id)?;
        for stream in &self.data.streams {
            let prefix = if stream.stream_id == "main" {
                String::new()
            } else {
                format!("[{}] ", stream.stream_id)
            };
            writeln!(
                f,
                "{}Turns: {} | Tokens: {}",
                prefix,
                stream.turns.len(),
                number::format_compact(stream.context_summary.current_tokens as i64)
            )?;
        }
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Compact: Multi-line header with clear key-value pairs + one-line per turn
        let session_id = self.data.session.session_id.replace(['\n', '\r'], "");
        writeln!(f, "Session:  {}", session_id)?;

        let provider = self.data.session.provider.replace(['\n', '\r'], "");
        write!(f, "Provider: {}", provider)?;
        if let Some(main) = self.main_stream() {
            write!(f, " | Turns: {}", main.turns.len())?;
            write!(
                f,
                " | Tokens: {}",
                number::format_compact(main.context_summary.current_tokens as i64)
            )?;
        }
        writeln!(f)?;

        if let Some(ref root) = self.data.session.project_root {
            // Normalize first (handle newlines), then truncate path
            let normalized = root.replace(['\n', '\r'], "");
            writeln!(f, "Project:  {}", text::truncate_path(&normalized, 60))?;
        } else {
            let hash_normalized = self.data.session.project_hash.replace(['\n', '\r'], "");
            let hash_prefix = if hash_normalized.len() >= 8 {
                &hash_normalized[..8]
            } else {
                &hash_normalized
            };
            writeln!(f, "Project:  {}... (hash only)", hash_prefix)?;
        }

        if let Some(ref ctx) = self.data.session.spawned_by {
            writeln!(f, "Spawned:  {}", format_spawn_context(ctx))?;
        }

        writeln!(f, "{}", "=".repeat(80))?;
        writeln!(f)?;

        if let Some(main) = self.main_stream() {
            self.write_stream_turns(f, main)?;

            // Show final context summary
            let tokens_display = if let Some(max) = main.context_summary.max_tokens {
                format!(
                    "Context: {} / {} ({:.1}%)",
                    number::format_compact(main.context_summary.current_tokens as i64),
                    number::format_compact(max as i64),
                    (main.context_summary.current_tokens as f64 / max as f64) * 100.0
                )
            } else {
                format!(
                    "Context: {}",
                    number::format_compact(main.context_summary.current_tokens as i64)
                )
            };
            writeln!(f, "{}", tokens_display)?;
        }

        for stream in self.extra_streams() {
            self.write_stream_heading(f, stream)?;
            self.write_stream_turns(f, stream)?;
        }

        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Standard: Same as Verbose (merged for simplicity)
        self.render_verbose(f)
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Verbose: Extended information with all metadata
        writeln!(f, "{}", "=".repeat(80))?;

        // Session ID
        writeln!(f, "Session ID:    {}", self.data.session.session_id)?;

        // Provider
        writeln!(f, "Provider:      {}", self.data.session.provider)?;

        // Project with full information
        if let Some(ref project_root) = self.data.session.project_root {
            let smart_path = text::shorten_home_path(project_root);
            writeln!(f, "Project:       {}", smart_path)?;
            writeln!(f, "               (full: {})", project_root)?;
            writeln!(f, "Project Hash:  {}", self.data.session.project_hash)?;
        } else {
            writeln!(f, "Project Hash:  {}", self.data.session.project_hash)?;
            writeln!(f, "               (no root path available)")?;
        }

        // Model
        if let Some(ref model) = self.data.session.model {
            writeln!(f, "Model:         {}", model)?;
        }

        // Spawn context (for subagent sessions stored in separate files)
        if let Some(ref ctx) = self.data.session.spawned_by {
            writeln!(f, "Spawned:       {}", format_spawn_context(ctx))?;
        }

        if let Some(main) = self.main_stream() {
            // Status
            writeln!(f, "Status:        {}", main.status)?;

            // Turns
            writeln!(f, "Turns:         {}", main.turns.len())?;

            // Tokens with context bar
            let tokens_display = if let Some(max) = main.context_summary.max_tokens {
                format!(
                    "{} / {} ({:.1}%)",
                    number::format_compact(main.context_summary.current_tokens as i64),
                    number::format_compact(max as i64),
                    (main.context_summary.current_tokens as f64 / max as f64) * 100.0
                )
            } else {
                number::format_compact(main.context_summary.current_tokens as i64)
            };
            writeln!(f, "Tokens:        {}", tokens_display)?;
            if let Some(max) = main.context_summary.max_tokens {
                let bar = display::build_progress_bar(
                    main.context_summary.current_tokens,
                    max,
                    CONTEXT_BAR_WIDTH_STANDARD,
                );
                writeln!(f, "               {}", bar)?;
            }

            // Start time and duration
            if let Some(ref start) = main.start_time {
                writeln!(f, "Started:       {}", start)?;
            }
            if let Some(ref dur) = main.duration {
                writeln!(f, "Duration:      {}", dur)?;
            }
        }

        // Streams (only when there is more than just the main conversation)
        if !self.extra_streams().is_empty() {
            writeln!(f, "Streams:       {}", self.data.streams.len())?;
        }

        // Log files
        if !self.data.session.log_files.is_empty() {
            writeln!(f, "Log Files:")?;
            for log_file in &self.data.session.log_files {
                writeln!(f, "               {}", log_file)?;
            }
        }

        writeln!(f, "{}", "=".repeat(80))?;
        writeln!(f)?;

        // Main stream turns
        if let Some(main) = self.main_stream() {
            self.write_stream_turns(f, main)?;
        }

        // Additional streams (sidechains/subagents)
        for stream in self.extra_streams() {
            self.write_stream_heading(f, stream)?;
            self.write_stream_turns(f, stream)?;
        }

        Ok(())
    }
}

impl<'a> fmt::Display for SessionDetailView<'a> {
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

use crate::presentation::view_models::TurnAnalysisViewModel;

pub struct TurnView<'a> {
    data: &'a TurnAnalysisViewModel,
    mode: ViewMode,
}

impl<'a> TurnView<'a> {
    pub fn new(data: &'a TurnAnalysisViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn extract_delta_indicator(&self) -> String {
        let delta_value = self.data.metrics.total_delta;

        if delta_value > DELTA_ALERT_THRESHOLD {
            " 🔺".to_string()
        } else if delta_value > DELTA_WARNING_THRESHOLD {
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
                let char_count = preview.chars().count();
                let duration_str = duration
                    .as_ref()
                    .map(|d| format!(" ({})", d))
                    .unwrap_or_default();

                if matches!(self.mode, ViewMode::Minimal) {
                    // Minimal only: show char count and hint
                    writeln!(
                        f,
                        "{} 🧠 Thinking{} ({} chars) 💡 --verbose to expand",
                        prefix, duration_str, char_count
                    )?;
                } else {
                    // Compact, Standard, Verbose: show full thinking content
                    writeln!(f, "{} 🧠 Thinking{}", prefix, duration_str)?;
                    if !preview.is_empty() {
                        self.write_thinking_content(f, preview, is_last)?;
                    }
                }
            }
            AgentStepViewModel::ToolCall {
                name,
                arguments,
                result,
                is_error,
                agent_id,
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
                self.write_truncated_result(f, result, agent_id.as_deref())?;
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
                let truncated =
                    text::truncate_multiline(text, MESSAGE_MAX_LINES, MESSAGE_PREVIEW_LENGTH);
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
        // Use multiline truncation for thinking previews
        let truncated =
            text::truncate_multiline(preview, THINKING_MAX_LINES, THINKING_PREVIEW_LENGTH);
        self.write_indented(f, &truncated, is_last, "   ")?;
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

    fn write_truncated_result(
        &self,
        f: &mut fmt::Formatter,
        result: &str,
        agent_id: Option<&str>,
    ) -> fmt::Result {
        let display_text = match (result.trim().is_empty(), agent_id) {
            // Empty result with agent_id: show sidechain link
            (true, Some(aid)) => format!("→ sidechain:{}", aid),
            // Empty result without agent_id: show (empty)
            (true, None) => text::format_empty(result),
            // Non-empty result with agent_id: show both
            (false, Some(aid)) => {
                let truncated = text::normalize_and_clean(result, TOOL_RESULT_LENGTH);
                format!("{} (→ sidechain:{})", truncated, aid)
            }
            // Non-empty result without agent_id: show truncated text
            (false, None) => text::normalize_and_clean(result, TOOL_RESULT_LENGTH),
        };
        writeln!(f, "{}", display_text)?;
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

        // Compact mode: show everything in one line per turn
        if matches!(self.mode, ViewMode::Compact) {
            let tool_count = self
                .data
                .steps
                .iter()
                .filter(|s| matches!(s, AgentStepViewModel::ToolCall { .. }))
                .count();

            let user_msg = text::normalize_and_clean(&self.data.user_query, 80);

            writeln!(
                f,
                "[Turn #{:02}] {} {} | {} tools | {}",
                self.data.turn_number, status_icon, status_label, tool_count, user_msg
            )?;
            return Ok(());
        }

        // Standard/Verbose modes: show full context information
        let delta_indicator = self.extract_delta_indicator();
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
            let bar = display::build_progress_bar(
                usage.current_tokens,
                usage.max_tokens,
                CONTEXT_BAR_WIDTH_COMPACT,
            );
            writeln!(
                f,
                "│ {} ({} / {})",
                bar,
                number::format_compact(usage.current_tokens as i64),
                number::format_compact(usage.max_tokens as i64)
            )?;
        }

        // Standard/Verbose modes: show full details
        writeln!(f, "│")?;

        // Calculate total items to display (user + steps)
        let total_items = 1 + self.data.steps.len();
        let mut current_index = 0;

        // User query
        current_index += 1;
        let is_last = current_index == total_items;
        let prefix = if is_last { "└──" } else { "├──" };
        writeln!(f, "{} 👤 User", prefix)?;
        let truncated_query =
            text::truncate_multiline(&self.data.user_query, QUERY_MAX_LINES, QUERY_DISPLAY_LENGTH);
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

        // Show spawned children (subagent sessions) if any
        if !self.data.spawned_children.is_empty() {
            write!(f, "🔀 Spawned: ")?;
            for (i, child) in self.data.spawned_children.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", child.session_id_short)?;
            }
            writeln!(f)?;
        }

        writeln!(f)?;

        Ok(())
    }
}

// --------------------------------------------------------
// Helper Functions
// --------------------------------------------------------

/// Format tool arguments for display
fn format_tool_args(tool_call: &agtrace_sdk::types::ToolCallPayload) -> String {
    use agtrace_sdk::types::ToolCallPayload;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::view_models::session::{
        ContextWindowSummary, FilterSummary, SessionDetailViewModel, SessionInfoViewModel,
        SessionListEntry, SessionListViewModel, SpawnContextViewModel, StreamAnalysisViewModel,
        TurnAnalysisViewModel, TurnMetrics,
    };

    fn make_stream(
        stream_id: &str,
        spawned_by: Option<SpawnContextViewModel>,
    ) -> StreamAnalysisViewModel {
        StreamAnalysisViewModel {
            stream_id: stream_id.to_string(),
            spawned_by,
            status: "Complete".to_string(),
            duration: None,
            start_time: None,
            context_summary: ContextWindowSummary {
                current_tokens: 1000,
                max_tokens: Some(10000),
            },
            turns: vec![],
        }
    }

    fn make_session_detail(streams: Vec<StreamAnalysisViewModel>) -> SessionDetailViewModel {
        SessionDetailViewModel {
            session: SessionInfoViewModel {
                session_id: "test-session-id".to_string(),
                provider: "test_provider".to_string(),
                project_hash: "test-project-hash-12345678".to_string(),
                project_root: Some("/test/project/root".to_string()),
                model: Some("test-model".to_string()),
                log_files: vec![],
                spawned_by: None,
            },
            streams,
        }
    }

    #[test]
    fn test_session_list_compact_empty() {
        let data = SessionListViewModel {
            sessions: vec![],
            total_count: 0,
            applied_filters: FilterSummary {
                project_filter: None,
                provider_filter: None,
                time_range: None,
                limit: 50,
            },
        };
        let view = SessionListView::new(&data, ViewMode::Compact);
        let output = format!("{}", view);
        assert!(output.contains("No sessions found"));
    }

    #[test]
    fn test_session_list_compact_with_session() {
        let session = SessionListEntry {
            id: "abc123def456".to_string(),
            provider: "test_provider".to_string(),
            project_hash: "hash123".to_string(),
            project_root: None,
            start_ts: Some("2025-12-24T12:00:00Z".to_string()),
            snippet: Some("Test snippet".to_string()),
        };
        let data = SessionListViewModel {
            sessions: vec![session],
            total_count: 1,
            applied_filters: FilterSummary {
                project_filter: None,
                provider_filter: None,
                time_range: None,
                limit: 50,
            },
        };
        let view = SessionListView::new(&data, ViewMode::Compact);
        let output = format!("{}", view);

        // Should show shortened ID (first 8 chars)
        assert!(output.contains("abc123de"));
        assert!(output.contains("test_provider"));
        assert!(output.contains("Test snippet"));
    }

    #[test]
    fn test_session_list_missing_snippet() {
        let session = SessionListEntry {
            id: "test123".to_string(),
            provider: "test".to_string(),
            project_hash: "hash".to_string(),
            project_root: None,
            start_ts: None,
            snippet: None,
        };
        let data = SessionListViewModel {
            sessions: vec![session],
            total_count: 1,
            applied_filters: FilterSummary {
                project_filter: None,
                provider_filter: None,
                time_range: None,
                limit: 50,
            },
        };
        let view = SessionListView::new(&data, ViewMode::Compact);
        let output = format!("{}", view);

        // Should show "--" instead of "(no snippet)"
        assert!(output.contains("--"));
        assert!(!output.contains("(no snippet)"));
    }

    #[test]
    fn test_delta_indicator_thresholds() {
        // Create a minimal turn for testing delta indicators
        let make_turn = |delta: u32| TurnAnalysisViewModel {
            turn_number: 1,
            timestamp: None,
            user_query: "test".to_string(),
            steps: vec![],
            metrics: TurnMetrics {
                input_tokens: 100,
                output_tokens: 50,
                cache_read_tokens: None,
                total_delta: delta,
            },
            prev_tokens: 0,
            current_tokens: delta,
            context_usage: None,
            is_heavy_load: false,
            spawned_children: vec![],
        };

        // Test below warning threshold
        let turn = make_turn(10_000);
        let view = TurnView::new(&turn, ViewMode::Standard);
        assert_eq!(view.extract_delta_indicator(), "");

        // Test above warning threshold
        let turn = make_turn(25_000);
        let view = TurnView::new(&turn, ViewMode::Standard);
        assert_eq!(view.extract_delta_indicator(), " ⚡");

        // Test above alert threshold
        let turn = make_turn(60_000);
        let view = TurnView::new(&turn, ViewMode::Standard);
        assert_eq!(view.extract_delta_indicator(), " 🔺");
    }

    #[test]
    fn test_session_detail_compact() {
        let data = make_session_detail(vec![make_stream("main", None)]);
        let view = SessionDetailView::new(&data, ViewMode::Compact);
        let output = format!("{}", view);

        assert!(output.contains("test-session-id"));
        assert!(output.contains("test_provider"));
        assert!(output.contains("Turns: 0"));
    }

    #[test]
    fn test_session_detail_with_sidechains_renders_stream_sections() {
        let data = make_session_detail(vec![
            make_stream("main", None),
            make_stream(
                "sidechain:abc12345",
                Some(SpawnContextViewModel {
                    turn_index: 1,
                    step_index: 1,
                }),
            ),
        ]);
        let view = SessionDetailView::new(&data, ViewMode::Verbose);
        let output = format!("{}", view);

        assert!(output.contains("Streams:       2"));
        assert!(output.contains("Stream: sidechain:abc12345 (spawned by Turn #2, Step #2)"));
    }

    /// Regression test: a session with sidechains must serialize to a single
    /// valid JSON document (previously each stream was emitted as a separate
    /// document with text separators in between, producing invalid JSON).
    #[test]
    fn test_session_detail_with_sidechains_is_single_json_document() {
        use crate::presentation::view_models::CommandResultViewModel;

        let data = make_session_detail(vec![
            make_stream("main", None),
            make_stream(
                "sidechain:abc12345",
                Some(SpawnContextViewModel {
                    turn_index: 1,
                    step_index: 1,
                }),
            ),
            make_stream(
                "sidechain:def67890",
                Some(SpawnContextViewModel {
                    turn_index: 2,
                    step_index: 0,
                }),
            ),
        ]);
        let result = CommandResultViewModel::new(data);
        let output = serde_json::to_string_pretty(&result).unwrap();

        // The whole output must parse as ONE JSON document
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        let streams = parsed["content"]["streams"].as_array().unwrap();
        assert_eq!(streams.len(), 3);
        assert_eq!(streams[0]["stream_id"], "main");
        assert_eq!(streams[1]["spawned_by"]["turn_index"], 1);
    }

    #[test]
    fn test_empty_tool_result_display() {
        use crate::presentation::view_models::session::AgentStepViewModel;
        use agtrace_sdk::types::ToolCallPayload;

        let turn = TurnAnalysisViewModel {
            turn_number: 1,
            timestamp: None,
            user_query: "test".to_string(),
            steps: vec![AgentStepViewModel::ToolCall {
                name: "Bash".to_string(),
                arguments: ToolCallPayload::Generic {
                    name: "bash".to_string(),
                    provider_call_id: Some("test-id".to_string()),
                    arguments: serde_json::json!({"command": "mkdir test"}),
                },
                args_formatted: None,
                result: "".to_string(), // Empty result
                is_error: false,
                agent_id: None,
            }],
            metrics: TurnMetrics {
                input_tokens: 100,
                output_tokens: 50,
                cache_read_tokens: None,
                total_delta: 150,
            },
            prev_tokens: 0,
            current_tokens: 150,
            context_usage: None,
            is_heavy_load: false,
            spawned_children: vec![],
        };

        let view = TurnView::new(&turn, ViewMode::Standard);
        let output = format!("{}", view);

        // Empty result should show "(empty)"
        assert!(output.contains("(empty)"));
    }
}
