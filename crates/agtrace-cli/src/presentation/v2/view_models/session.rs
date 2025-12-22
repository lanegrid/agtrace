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
    pub is_heavy_load: bool,
    pub user_query: String,
    pub steps: Vec<AgentStepViewModel>,
    pub metrics: TurnMetrics,
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

        writeln!(
            f,
            "[Turn #{:02}] {} {}  (Context: {})",
            self.turn_number, status_icon, status_label, self.context_transition
        )?;

        writeln!(f, "  👤 User: \"{}\"", self.user_query)?;
        writeln!(f, "  {}", "─".repeat(76))?;

        // Steps
        for step in &self.steps {
            match step {
                AgentStepViewModel::Thinking { duration, preview } => {
                    if let Some(dur) = duration {
                        writeln!(f, "  🧠 Thinking ({})", dur)?;
                    } else {
                        writeln!(f, "  🧠 Thinking")?;
                    }
                    if !preview.is_empty() {
                        writeln!(f, "     {}", preview)?;
                    }
                }
                AgentStepViewModel::ToolCall {
                    name,
                    args,
                    result,
                    is_error,
                } => {
                    writeln!(f, "  🔧 Tool: {} (args: {})", name, args)?;
                    if *is_error {
                        writeln!(f, "     ↳ ❌ Error: {}", result)?;
                    } else {
                        writeln!(f, "     ↳ 📝 Result: {}", result)?;
                    }
                }
                AgentStepViewModel::Message { text } => {
                    writeln!(f, "  💬 Msg: {}", text)?;
                }
                AgentStepViewModel::SystemEvent { description } => {
                    writeln!(f, "  ℹ️  {}", description)?;
                }
            }
        }

        writeln!(f, "  {}", "─".repeat(76))?;
        writeln!(
            f,
            "  📊 Stats: {} (In: {}, Out: {}{})",
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
