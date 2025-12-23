use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

use super::{CreateView, ViewMode};

#[derive(Debug, Serialize)]
pub struct DiagnoseResultsViewModel {
    pub results: Vec<DiagnoseResultViewModel>,
}

impl CreateView for DiagnoseResultsViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(DiagnoseResultsView { data: self })
    }
}

struct DiagnoseResultsView<'a> {
    data: &'a DiagnoseResultsViewModel,
}

impl<'a> fmt::Display for DiagnoseResultsView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for result in &self.data.results {
            write!(f, "{}", DiagnoseResultView { data: result })?;
        }
        Ok(())
    }
}

impl fmt::Display for DiagnoseResultsViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", DiagnoseResultsView { data: self })
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

impl CreateView for DiagnoseResultViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(DiagnoseResultView { data: self })
    }
}

struct DiagnoseResultView<'a> {
    data: &'a DiagnoseResultViewModel,
}

impl<'a> fmt::Display for DiagnoseResultView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\nProvider: {}", self.data.provider_name)?;
        writeln!(
            f,
            "  Files analyzed: {} ({} successful, {} failed)",
            self.data.total_files,
            self.data.successful,
            self.data.failures.values().map(|v| v.len()).sum::<usize>()
        )?;

        if !self.data.failures.is_empty() {
            writeln!(f, "\n  Failures by reason:")?;
            for (reason, examples) in &self.data.failures {
                writeln!(f, "    • {} ({} files)", reason, examples.len())?;
                for (i, example) in examples.iter().take(3).enumerate() {
                    writeln!(f, "      [{}] {}", i + 1, example.path)?;
                }
                if examples.len() > 3 {
                    writeln!(f, "      ... and {} more", examples.len() - 3)?;
                }
            }
        }

        Ok(())
    }
}

impl fmt::Display for DiagnoseResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", DiagnoseResultView { data: self })
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

impl CreateView for DoctorCheckResultViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(DoctorCheckResultView { data: self })
    }
}

struct DoctorCheckResultView<'a> {
    data: &'a DoctorCheckResultViewModel,
}

impl<'a> fmt::Display for DoctorCheckResultView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "File: {}", self.data.file_path)?;
        writeln!(f, "Provider: {}", self.data.provider_name)?;
        match self.data.status {
            CheckStatus::Success => {
                writeln!(f, "Status: ✓ Valid")?;
                writeln!(f, "Events parsed: {}", self.data.event_count)?;
            }
            CheckStatus::Failure => {
                writeln!(f, "Status: ✗ Failed")?;
                if let Some(err) = &self.data.error_message {
                    writeln!(f, "Error: {}", err)?;
                }
            }
        }

        Ok(())
    }
}

impl fmt::Display for DoctorCheckResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", DoctorCheckResultView { data: self })
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

impl CreateView for InspectResultViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(InspectResultView { data: self })
    }
}

struct InspectResultView<'a> {
    data: &'a InspectResultViewModel,
}

impl<'a> fmt::Display for InspectResultView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "File: {}", self.data.file_path)?;
        writeln!(
            f,
            "Lines: 1-{} (total: {} lines)",
            self.data.shown_lines.min(self.data.total_lines),
            self.data.total_lines
        )?;
        writeln!(f, "{}", "─".repeat(40))?;

        for line in &self.data.lines {
            writeln!(f, "{:>6}  {}", line.number, line.content)?;
        }

        writeln!(f, "{}", "─".repeat(40))?;

        Ok(())
    }
}

impl fmt::Display for InspectResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", InspectResultView { data: self })
    }
}
