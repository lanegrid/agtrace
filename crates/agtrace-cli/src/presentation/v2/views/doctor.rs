use std::fmt;

use crate::presentation::v2::view_models::{
    CheckStatus, DiagnoseResultViewModel, DiagnoseResultsViewModel, DoctorCheckResultViewModel,
    InspectResultViewModel,
};

// --------------------------------------------------------
// Diagnose Results View
// --------------------------------------------------------

pub struct DiagnoseResultsView<'a> {
    pub data: &'a DiagnoseResultsViewModel,
}

impl<'a> DiagnoseResultsView<'a> {
    pub fn new(data: &'a DiagnoseResultsViewModel) -> Self {
        Self { data }
    }
}

impl<'a> fmt::Display for DiagnoseResultsView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for result in &self.data.results {
            write!(f, "{}", DiagnoseResultView::new(result))?;
        }
        Ok(())
    }
}

// --------------------------------------------------------
// Diagnose Result View
// --------------------------------------------------------

pub struct DiagnoseResultView<'a> {
    data: &'a DiagnoseResultViewModel,
}

impl<'a> DiagnoseResultView<'a> {
    pub fn new(data: &'a DiagnoseResultViewModel) -> Self {
        Self { data }
    }
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

// --------------------------------------------------------
// Doctor Check Result View
// --------------------------------------------------------

pub struct DoctorCheckResultView<'a> {
    data: &'a DoctorCheckResultViewModel,
}

impl<'a> DoctorCheckResultView<'a> {
    pub fn new(data: &'a DoctorCheckResultViewModel) -> Self {
        Self { data }
    }
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

// --------------------------------------------------------
// Inspect Result View
// --------------------------------------------------------

pub struct InspectResultView<'a> {
    data: &'a InspectResultViewModel,
}

impl<'a> InspectResultView<'a> {
    pub fn new(data: &'a InspectResultViewModel) -> Self {
        Self { data }
    }
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
