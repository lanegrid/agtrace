use serde::Serialize;
use std::fmt;

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

fn build_progress_bar_string(current: u32, max: u32, percent: f64) -> String {
    let bar_width = 20;
    let filled = ((percent / 100.0) * bar_width as f64) as usize;
    let filled = filled.min(bar_width);
    let empty = bar_width - filled;

    format!(
        "[{}{}] {:.1}% ({} / {})",
        "█".repeat(filled),
        "░".repeat(empty),
        percent,
        format_tokens(current as i64),
        format_tokens(max as i64)
    )
}

fn format_tokens(count: i64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

impl fmt::Display for SessionListViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::presentation::formatters::session_list::SessionEntry;
        use crate::presentation::formatters::SessionListView;

        if self.sessions.is_empty() {
            writeln!(f, "No sessions found.")?;
            if let Some(ref project) = self.applied_filters.project_filter {
                writeln!(f, "Project filter: {}", project)?;
            }
            if let Some(ref source) = self.applied_filters.source_filter {
                writeln!(f, "Source filter: {}", source)?;
            }
            return Ok(());
        }

        let entries: Vec<SessionEntry> = self
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

        if self.applied_filters.project_filter.is_some()
            || self.applied_filters.source_filter.is_some()
            || self.applied_filters.time_range.is_some()
        {
            writeln!(f)?;
            writeln!(f, "Filters applied:")?;
            if let Some(ref project) = self.applied_filters.project_filter {
                writeln!(f, "  Project: {}", project)?;
            }
            if let Some(ref source) = self.applied_filters.source_filter {
                writeln!(f, "  Source: {}", source)?;
            }
            if let Some(ref range) = self.applied_filters.time_range {
                writeln!(f, "  Time range: {}", range)?;
            }
        }

        Ok(())
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
    pub progress_bar: String,
    pub usage_percent: String,
    pub usage_fraction: String,
    pub warning: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TurnAnalysisViewModel {
    pub turn_number: usize,
    pub timestamp: Option<String>,
    pub context_transition: String,
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
        args: String,
        result: String,
        is_error: bool,
    },
    ToolCallSequence {
        name: String,
        count: usize,
        sample_args: String,
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
    pub total_delta: String,
    pub input: String,
    pub output: String,
    pub cache_read: Option<String>,
}

impl fmt::Display for SessionAnalysisViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Header
        writeln!(f, "{}", "=".repeat(80))?;
        write!(f, "SESSION: {}", self.header.session_id)?;
        if let Some(ref model) = self.header.model {
            write!(f, " ({})", model)?;
        }
        writeln!(f)?;
        writeln!(f, "STATUS:  {}", self.header.status)?;
        writeln!(f, "CONTEXT: {}", self.context_summary.progress_bar)?;
        writeln!(f, "{}", "=".repeat(80))?;
        writeln!(f)?;

        // Turns
        for turn in &self.turns {
            write!(f, "{}", turn)?;
        }

        Ok(())
    }
}

impl fmt::Display for TurnAnalysisViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let status_icon = if self.is_heavy_load { "⚠️" } else { "🟢" };
        let status_label = if self.is_heavy_load {
            "Heavy Load"
        } else {
            "Normal"
        };

        // Extract delta for visual indicator
        let delta_indicator = self.extract_delta_indicator();

        writeln!(
            f,
            "[Turn #{:02}] {} {}  (Context: {}{})",
            self.turn_number, status_icon, status_label, self.context_transition, delta_indicator
        )?;

        // Show progress bar if context usage data is available
        if let Some(ref usage) = self.context_usage {
            let progress_bar =
                build_progress_bar_string(usage.current_tokens, usage.max_tokens, usage.percentage);
            writeln!(f, "│ {}", progress_bar)?;
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
        self.write_indented(f, &self.user_query, is_last, "   ")?;

        // Steps
        for step in &self.steps {
            current_index += 1;
            let is_last = current_index == total_items;
            self.write_step(f, step, is_last)?;
        }

        writeln!(f)?;
        writeln!(
            f,
            "📊 Stats: {} (In: {}, Out: {}{})",
            self.metrics.total_delta,
            self.metrics.input,
            self.metrics.output,
            self.metrics
                .cache_read
                .as_ref()
                .map(|c| format!(", Cache: {}", c))
                .unwrap_or_default()
        )?;
        writeln!(f)?;

        Ok(())
    }
}

impl TurnAnalysisViewModel {
    fn extract_delta_indicator(&self) -> String {
        // Extract numeric value from total_delta (e.g., "+57.2k" -> 57200)
        let delta_str = self.metrics.total_delta.trim_start_matches('+');
        let delta_value = if delta_str.ends_with('k') {
            delta_str
                .trim_end_matches('k')
                .parse::<f64>()
                .unwrap_or(0.0)
                * 1000.0
        } else if delta_str.ends_with('M') {
            delta_str
                .trim_end_matches('M')
                .parse::<f64>()
                .unwrap_or(0.0)
                * 1_000_000.0
        } else {
            delta_str.parse::<f64>().unwrap_or(0.0)
        };

        if delta_value > 50_000.0 {
            " 🔺".to_string()
        } else if delta_value > 20_000.0 {
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
                args,
                result,
                is_error,
            } => {
                writeln!(f, "{} 🔧 Tool: {}", prefix, name)?;
                self.write_indented(f, args, is_last, "   ")?;

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
                sample_args,
                has_errors,
            } => {
                let status = if *has_errors { "⚠️" } else { "✓" };
                writeln!(
                    f,
                    "{} 🔧 Tool: {} (x{} calls) {}",
                    prefix, name, count, status
                )?;
                self.write_indented(f, sample_args, is_last, "   ")?;
            }
            AgentStepViewModel::Message { text } => {
                writeln!(f, "{} 💬 Message", prefix)?;
                self.write_indented(f, text, is_last, "   ")?;
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

        // Show first line prominently, rest dimmed
        let lines: Vec<&str> = preview.lines().collect();
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
        let char_count = result.chars().count();
        if char_count > 100 {
            writeln!(f, "[Truncated: {} chars]", char_count)?;
        } else {
            writeln!(f, "{}", result)?;
        }
        Ok(())
    }
}
