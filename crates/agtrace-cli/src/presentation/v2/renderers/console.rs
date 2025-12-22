use anyhow::Result;
use owo_colors::OwoColorize;
use serde::Serialize;
use std::fmt::Display;

use super::traits::Renderer;
use crate::presentation::v2::view_models::CommandResultViewModel;

pub struct ConsoleRenderer {
    json_mode: bool,
}

impl ConsoleRenderer {
    pub fn new(json_mode: bool) -> Self {
        Self { json_mode }
    }
}

impl Renderer for ConsoleRenderer {
    fn render<T>(&self, result: CommandResultViewModel<T>) -> Result<()>
    where
        T: Serialize + Display + Send + Sync,
    {
        if self.json_mode {
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(());
        }

        if let Some(badge) = &result.badge {
            println!("{} {}", badge.icon(), badge.label.bold());
            println!();
        }

        print!("{}", result.content);

        if !result.suggestions.is_empty() {
            println!("\n{}", "💡 Tips:".yellow().bold());
            for tip in &result.suggestions {
                print!("  • {}", tip.description);
                if let Some(cmd) = &tip.command {
                    print!(": {}", cmd.cyan());
                }
                println!();
            }
        }

        Ok(())
    }
}
