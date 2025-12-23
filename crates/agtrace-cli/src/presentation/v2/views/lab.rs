use std::fmt;

use crate::presentation::v2::view_models::{
    CreateView, LabExportViewModel, LabStatsViewModel, ViewMode,
};

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
