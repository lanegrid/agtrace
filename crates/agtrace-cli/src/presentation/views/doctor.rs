use std::fmt;

use crate::presentation::view_models::{
    CheckStatus, DiagnoseResultViewModel, DiagnoseResultsViewModel, DoctorCheckResultViewModel,
    InspectResultViewModel, ViewMode,
};

// --------------------------------------------------------
// Diagnose Results View
// --------------------------------------------------------

pub struct DiagnoseResultsView<'a> {
    pub data: &'a DiagnoseResultsViewModel,
    mode: ViewMode,
}

impl<'a> DiagnoseResultsView<'a> {
    pub fn new(data: &'a DiagnoseResultsViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Minimal: Just provider names and total counts
        for result in &self.data.results {
            let failures: usize = result.failures.values().map(|v| v.len()).sum();
            writeln!(
                f,
                "{}: {}/{}",
                result.provider_name, result.successful, failures
            )?;
        }
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Compact: One line per provider with summary
        for result in &self.data.results {
            let failures: usize = result.failures.values().map(|v| v.len()).sum();
            writeln!(
                f,
                "{}: {} files ({} ok, {} failed)",
                result.provider_name, result.total_files, result.successful, failures
            )?;
        }
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Standard: Current behavior
        for result in &self.data.results {
            write!(f, "{}", DiagnoseResultView::new(result, self.mode))?;
        }
        Ok(())
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Verbose: Standard + all failure examples (no truncation)
        for result in &self.data.results {
            write!(f, "{}", DiagnoseResultView::new(result, ViewMode::Verbose))?;
        }
        Ok(())
    }
}

impl<'a> fmt::Display for DiagnoseResultsView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}

// --------------------------------------------------------
// Diagnose Result View
// --------------------------------------------------------

pub struct DiagnoseResultView<'a> {
    data: &'a DiagnoseResultViewModel,
    mode: ViewMode,
}

impl<'a> DiagnoseResultView<'a> {
    pub fn new(data: &'a DiagnoseResultViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let failures: usize = self.data.failures.values().map(|v| v.len()).sum();
        writeln!(
            f,
            "{}: {}/{}",
            self.data.provider_name, self.data.successful, failures
        )?;
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let failures: usize = self.data.failures.values().map(|v| v.len()).sum();
        writeln!(
            f,
            "{}: {} files ({} ok, {} failed)",
            self.data.provider_name, self.data.total_files, self.data.successful, failures
        )?;
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
                // Verbose: Show all examples, not just first 3
                for (i, example) in examples.iter().enumerate() {
                    writeln!(f, "      [{}] {}", i + 1, example.path)?;
                }
            }
        }

        Ok(())
    }
}

impl<'a> fmt::Display for DiagnoseResultView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}

// --------------------------------------------------------
// Doctor Check Result View
// --------------------------------------------------------

pub struct DoctorCheckResultView<'a> {
    data: &'a DoctorCheckResultViewModel,
    mode: ViewMode,
}

impl<'a> DoctorCheckResultView<'a> {
    pub fn new(data: &'a DoctorCheckResultViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.data.status {
            CheckStatus::Success => writeln!(f, "✓")?,
            CheckStatus::Failure => writeln!(f, "✗")?,
        }
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.data.status {
            CheckStatus::Success => writeln!(
                f,
                "✓ {} ({} events)",
                self.data.file_path, self.data.event_count
            )?,
            CheckStatus::Failure => writeln!(f, "✗ {}", self.data.file_path)?,
        }
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
                    // Verbose: Could show more details here if available
                }
            }
        }

        Ok(())
    }
}

impl<'a> fmt::Display for DoctorCheckResultView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}

// --------------------------------------------------------
// Inspect Result View
// --------------------------------------------------------

pub struct InspectResultView<'a> {
    data: &'a InspectResultViewModel,
    mode: ViewMode,
}

impl<'a> InspectResultView<'a> {
    pub fn new(data: &'a InspectResultViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.data.file_path)?;
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "{} ({} lines)",
            self.data.file_path, self.data.total_lines
        )?;
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

impl<'a> fmt::Display for InspectResultView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}
