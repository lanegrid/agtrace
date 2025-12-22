use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct DiagnoseResultsViewModel {
    pub results: Vec<DiagnoseResultViewModel>,
}

impl fmt::Display for DiagnoseResultsViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for result in &self.results {
            write!(f, "{}", result)?;
        }
        Ok(())
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

impl fmt::Display for DiagnoseResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\nProvider: {}", self.provider_name)?;
        writeln!(
            f,
            "  Files analyzed: {} ({} successful, {} failed)",
            self.total_files,
            self.successful,
            self.failures.values().map(|v| v.len()).sum::<usize>()
        )?;

        if !self.failures.is_empty() {
            writeln!(f, "\n  Failures by reason:")?;
            for (reason, examples) in &self.failures {
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

impl fmt::Display for DoctorCheckResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "File: {}", self.file_path)?;
        writeln!(f, "Provider: {}", self.provider_name)?;
        match self.status {
            CheckStatus::Success => {
                writeln!(f, "Status: ✓ Valid")?;
                writeln!(f, "Events parsed: {}", self.event_count)?;
            }
            CheckStatus::Failure => {
                writeln!(f, "Status: ✗ Failed")?;
                if let Some(err) = &self.error_message {
                    writeln!(f, "Error: {}", err)?;
                }
            }
        }

        Ok(())
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

impl fmt::Display for InspectResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "File: {}", self.file_path)?;
        writeln!(
            f,
            "Lines: 1-{} (total: {} lines)",
            self.shown_lines.min(self.total_lines),
            self.total_lines
        )?;
        writeln!(f, "{}", "─".repeat(40))?;

        for line in &self.lines {
            writeln!(f, "{:>6}  {}", line.number, line.content)?;
        }

        writeln!(f, "{}", "─".repeat(40))?;

        Ok(())
    }
}
