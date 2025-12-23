use std::fmt;

use crate::presentation::v2::view_models::{IndexMode, IndexResultViewModel, VacuumResultViewModel};

// --------------------------------------------------------
// Index Result View
// --------------------------------------------------------

pub struct IndexResultView<'a> {
    data: &'a IndexResultViewModel,
}

impl<'a> IndexResultView<'a> {
    pub fn new(data: &'a IndexResultViewModel) -> Self {
        Self { data }
    }
}

impl<'a> fmt::Display for IndexResultView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

// --------------------------------------------------------
// Vacuum Result View
// --------------------------------------------------------

pub struct VacuumResultView<'a> {
    _data: &'a VacuumResultViewModel,
}

impl<'a> VacuumResultView<'a> {
    pub fn new(data: &'a VacuumResultViewModel) -> Self {
        Self { _data: data }
    }
}

impl<'a> fmt::Display for VacuumResultView<'a> {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}
