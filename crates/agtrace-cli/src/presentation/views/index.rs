use std::fmt;

use crate::presentation::view_models::{
    IndexInfoViewModel, IndexMode, IndexResultViewModel, VacuumResultViewModel, ViewMode,
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

// --------------------------------------------------------
// Index Info View
// --------------------------------------------------------

pub struct IndexInfoView<'a> {
    data: &'a IndexInfoViewModel,
    mode: ViewMode,
}

impl<'a> IndexInfoView<'a> {
    pub fn new(data: &'a IndexInfoViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn format_size(&self, bytes: u64) -> String {
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.data.data_dir.display())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Data dir: {}", self.data.data_dir.display())?;
        writeln!(f, "Database: {}", self.data.db_path.display())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f)?;
        writeln!(f, "Data directory: {}", self.data.data_dir.display())?;
        writeln!(f, "Database:       {}", self.data.db_path.display())?;
        writeln!(f, "Config:         {}", self.data.config_path.display())?;
        writeln!(f)?;
        if self.data.db_exists {
            writeln!(
                f,
                "Database size:  {}",
                self.format_size(self.data.db_size_bytes)
            )?;
        } else {
            writeln!(f, "Database size:  (not created)")?;
        }
        Ok(())
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.render_standard(f)
    }
}

impl<'a> fmt::Display for IndexInfoView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}
