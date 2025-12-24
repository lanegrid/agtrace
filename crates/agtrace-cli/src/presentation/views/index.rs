use std::fmt;

use crate::presentation::view_models::{
    IndexMode, IndexResultViewModel, VacuumResultViewModel, ViewMode,
};

// --------------------------------------------------------
// Index Result View
// --------------------------------------------------------

pub struct IndexResultView<'a> {
    data: &'a IndexResultViewModel,
    mode: ViewMode,
}

impl<'a> IndexResultView<'a> {
    pub fn new(data: &'a IndexResultViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.data.total_sessions)?;
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.data.mode {
            IndexMode::Update => {
                if self.data.total_sessions > 0 {
                    writeln!(
                        f,
                        "{} sessions ({} scanned, {} skipped)",
                        self.data.total_sessions, self.data.scanned_files, self.data.skipped_files
                    )?;
                } else {
                    writeln!(f, "No new sessions")?;
                }
            }
            IndexMode::Rebuild => {
                writeln!(
                    f,
                    "{} sessions ({} scanned)",
                    self.data.total_sessions, self.data.scanned_files
                )?;
            }
        }
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.data.mode {
            IndexMode::Update => {
                if self.data.total_sessions > 0 {
                    writeln!(
                        f,
                        "\nIndexed {} session(s) ({} files scanned, {} skipped)",
                        self.data.total_sessions, self.data.scanned_files, self.data.skipped_files
                    )?;
                } else {
                    writeln!(f, "\nNo new sessions found.")?;
                }
            }
            IndexMode::Rebuild => {
                writeln!(
                    f,
                    "\nRebuilt index: {} session(s) ({} files scanned)",
                    self.data.total_sessions, self.data.scanned_files
                )?;
            }
        }
        Ok(())
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.data.mode {
            IndexMode::Update => {
                if self.data.total_sessions > 0 {
                    writeln!(
                        f,
                        "\nIndexed {} session(s) ({} files scanned, {} skipped)",
                        self.data.total_sessions, self.data.scanned_files, self.data.skipped_files
                    )?;
                } else {
                    writeln!(f, "\nNo new sessions found.")?;
                }
            }
            IndexMode::Rebuild => {
                writeln!(
                    f,
                    "\nRebuilt index: {} session(s) ({} files scanned)",
                    self.data.total_sessions, self.data.scanned_files
                )?;
            }
        }
        Ok(())
    }
}

impl<'a> fmt::Display for IndexResultView<'a> {
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
// Vacuum Result View
// --------------------------------------------------------

pub struct VacuumResultView<'a> {
    _data: &'a VacuumResultViewModel,
    mode: ViewMode,
}

impl<'a> VacuumResultView<'a> {
    pub fn new(data: &'a VacuumResultViewModel, mode: ViewMode) -> Self {
        Self { _data: data, mode }
    }

    fn render_minimal(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }

    fn render_compact(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }

    fn render_standard(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }

    fn render_verbose(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

impl<'a> fmt::Display for VacuumResultView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}
