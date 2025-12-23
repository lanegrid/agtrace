use std::fmt;

use crate::presentation::v2::view_models::{
    CreateView, LabExportViewModel, LabGrepViewModel, LabStatsViewModel, ViewMode,
};
use owo_colors::OwoColorize;

impl CreateView for LabExportViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(LabExportView { data: self })
    }
}

struct LabExportView<'a> {
    data: &'a LabExportViewModel,
}

impl<'a> fmt::Display for LabExportView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Exported {} events to {}",
            self.data.exported_count, self.data.output_path
        )
    }
}

impl CreateView for LabStatsViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(LabStatsView { data: self })
    }
}

struct LabStatsView<'a> {
    data: &'a LabStatsViewModel,
}

impl<'a> fmt::Display for LabStatsView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Analyzing {} sessions...", self.data.total_sessions)?;

        writeln!(f, "\n=== ToolCall Statistics by Provider ===")?;
        for provider_stats in &self.data.providers {
            writeln!(f, "\n{}", "=".repeat(80))?;
            writeln!(f, "Provider: {}", provider_stats.provider_name)?;
            writeln!(f, "{}", "=".repeat(80))?;
            for tool_entry in &provider_stats.tools {
                writeln!(
                    f,
                    "\n  Tool: {} (count: {})",
                    tool_entry.tool_name, tool_entry.count
                )?;
                if let Some(sample) = &tool_entry.sample {
                    writeln!(f, "    Input:")?;
                    writeln!(f, "      {}", sample.arguments)?;
                    if let Some(result) = &sample.result {
                        writeln!(f, "    Output:")?;
                        writeln!(f, "      {}", result)?;
                    }
                }
            }

            if !provider_stats.classifications.is_empty() {
                writeln!(f, "\n  Classifications:")?;
                for classification in &provider_stats.classifications {
                    write!(f, "    - {}", classification.tool_name)?;
                    if let Some(origin) = &classification.origin {
                        write!(f, " (origin: {})", origin)?;
                    }
                    if let Some(kind) = &classification.kind {
                        write!(f, " [{}]", kind)?;
                    }
                    writeln!(f)?;
                }
            }
        }

        Ok(())
    }
}

impl CreateView for LabGrepViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(LabGrepView { data: self })
    }
}

struct LabGrepView<'a> {
    data: &'a LabGrepViewModel,
}

impl<'a> fmt::Display for LabGrepView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Searching for pattern '{}'...", self.data.pattern.cyan())?;
        writeln!(f, "Found {} matches:\n", self.data.matches.len())?;

        for (i, event) in self.data.matches.iter().enumerate() {
            writeln!(f, "{}", "=".repeat(80).bright_black())?;
            writeln!(
                f,
                "Match #{} | Session: {} | Stream: {:?}",
                i + 1,
                event.session_id.yellow(),
                event.stream_id
            )?;

            if self.data.json_output {
                let json = serde_json::to_string_pretty(&event.payload)
                    .unwrap_or_else(|_| "Failed to serialize payload".to_string());
                writeln!(f, "{}", json)?;
            } else {
                // Simple text format
                match &event.payload {
                    crate::presentation::v2::view_models::EventPayloadViewModel::User { text } => {
                        writeln!(f, "User: {}", text)?;
                    }
                    crate::presentation::v2::view_models::EventPayloadViewModel::Reasoning {
                        text,
                    } => {
                        writeln!(f, "Reasoning: {}", text)?;
                    }
                    crate::presentation::v2::view_models::EventPayloadViewModel::ToolCall {
                        name,
                        arguments,
                    } => {
                        writeln!(f, "ToolCall: {}", name)?;
                        let args_json = serde_json::to_string_pretty(arguments)
                            .unwrap_or_else(|_| "Invalid JSON".to_string());
                        writeln!(f, "  Arguments: {}", args_json)?;
                    }
                    crate::presentation::v2::view_models::EventPayloadViewModel::ToolResult {
                        output,
                        is_error,
                    } => {
                        writeln!(f, "ToolResult (error={}): {}", is_error, output)?;
                    }
                    crate::presentation::v2::view_models::EventPayloadViewModel::Message {
                        text,
                    } => {
                        writeln!(f, "Message: {}", text)?;
                    }
                    crate::presentation::v2::view_models::EventPayloadViewModel::TokenUsage {
                        input,
                        output,
                        total,
                        cache_creation,
                        cache_read,
                    } => {
                        writeln!(
                            f,
                            "TokenUsage: in={}, out={}, total={}, cache_create={:?}, cache_read={:?}",
                            input, output, total, cache_creation, cache_read
                        )?;
                    }
                    crate::presentation::v2::view_models::EventPayloadViewModel::Notification {
                        text,
                        level,
                    } => {
                        writeln!(f, "Notification [{:?}]: {}", level, text)?;
                    }
                }
            }
        }

        Ok(())
    }
}
