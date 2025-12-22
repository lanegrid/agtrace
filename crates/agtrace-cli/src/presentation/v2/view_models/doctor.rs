use serde::Serialize;
use std::collections::HashMap;

use crate::presentation::v2::renderers::ConsolePresentable;

#[derive(Debug, Serialize)]
pub struct DiagnoseResultsViewModel {
    pub results: Vec<DiagnoseResultViewModel>,
}

impl ConsolePresentable for DiagnoseResultsViewModel {
    fn render_console(&self) {
        for result in &self.results {
            result.render_console();
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DiagnoseResultViewModel {
    pub provider_name: String,
    pub total_files: usize,
    pub successful: usize,
    pub failures: HashMap<String, Vec<FailureExample>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct FailureExample {
    pub path: String,
    pub reason: String,
}

impl DiagnoseResultViewModel {
    fn render_console(&self) {
        println!("\nProvider: {}", self.provider_name);
        println!(
            "  Files analyzed: {} ({} successful, {} failed)",
            self.total_files,
            self.successful,
            self.failures.values().map(|v| v.len()).sum::<usize>()
        );

        if !self.failures.is_empty() {
            println!("\n  Failures by reason:");
            for (reason, examples) in &self.failures {
                println!("    • {} ({} files)", reason, examples.len());
                for (i, example) in examples.iter().take(3).enumerate() {
                    println!("      [{}] {}", i + 1, example.path);
                }
                if examples.len() > 3 {
                    println!("      ... and {} more", examples.len() - 3);
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DoctorCheckResultViewModel {
    pub file_path: String,
    pub provider_name: String,
    pub status: CheckStatus,
    pub event_count: usize,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Success,
    Failure,
}

impl ConsolePresentable for DoctorCheckResultViewModel {
    fn render_console(&self) {
        println!("File: {}", self.file_path);
        println!("Provider: {}", self.provider_name);
        match self.status {
            CheckStatus::Success => {
                println!("Status: ✓ Valid");
                println!("Events parsed: {}", self.event_count);
            }
            CheckStatus::Failure => {
                println!("Status: ✗ Failed");
                if let Some(err) = &self.error_message {
                    println!("Error: {}", err);
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InspectResultViewModel {
    pub file_path: String,
    pub total_lines: usize,
    pub shown_lines: usize,
    pub lines: Vec<InspectLine>,
}

#[derive(Debug, Serialize)]
pub struct InspectLine {
    pub number: usize,
    pub content: String,
}

impl ConsolePresentable for InspectResultViewModel {
    fn render_console(&self) {
        println!("File: {}", self.file_path);
        println!(
            "Lines: 1-{} (total: {} lines)",
            self.shown_lines.min(self.total_lines),
            self.total_lines
        );
        println!("{}", "─".repeat(40));

        for line in &self.lines {
            println!("{:>6}  {}", line.number, line.content);
        }

        println!("{}", "─".repeat(40));
    }
}
