use anyhow::Result;
use owo_colors::OwoColorize;
use serde::Serialize;

use super::traits::Renderer;
use crate::presentation::view_models::{
    CommandResultViewModel, CreateView, OutputFormat, ViewMode,
};

pub struct ConsoleRenderer {
    format: OutputFormat,
    mode: ViewMode,
}

impl ConsoleRenderer {
    pub fn new(format: OutputFormat, mode: ViewMode) -> Self {
        Self { format, mode }
    }

    /// Convenience constructor for JSON output (mode is ignored in JSON)
    pub fn json() -> Self {
        Self::new(OutputFormat::Json, ViewMode::default())
    }

    /// Convenience constructor for text output with default mode
    pub fn text() -> Self {
        Self::new(OutputFormat::Text, ViewMode::default())
    }
}

impl Renderer for ConsoleRenderer {
    fn render<T>(&self, result: CommandResultViewModel<T>) -> Result<()>
    where
        T: Serialize + CreateView + Send + Sync,
    {
        match self.format {
            OutputFormat::Json => {
                // JSON always outputs the full ViewModel, ignoring ViewMode
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            OutputFormat::Text => {
                // Minimal mode: ID only, no decorations (for scripting)
                let is_minimal = self.mode == ViewMode::Minimal;

                // Text rendering uses CreateView to generate mode-specific display
                if !is_minimal {
                    if let Some(badge) = &result.badge {
                        println!("{} {}", badge.icon(), badge.label.bold());
                        println!();
                    }
                }

                let view = result.content.create_view(self.mode);
                print!("{}", view);

                if !is_minimal && !result.suggestions.is_empty() {
                    println!("\n{}", "💡 Tips:".yellow().bold());
                    for tip in &result.suggestions {
                        print!("  • {}", tip.description);
                        if let Some(cmd) = &tip.command {
                            print!(": {}", cmd.cyan());
                        }
                        println!();
                    }
                }
            }
        }

        Ok(())
    }
}
