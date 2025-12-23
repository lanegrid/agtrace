use serde::Serialize;
use std::fmt;

use super::{CreateView, ViewMode};

#[derive(Debug, Serialize)]
pub struct IndexResultViewModel {
    pub total_sessions: usize,
    pub scanned_files: usize,
    pub skipped_files: usize,
    pub mode: IndexMode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexMode {
    Update,
    Rebuild,
}

impl CreateView for IndexResultViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(IndexResultView { data: self })
    }
}

struct IndexResultView<'a> {
    data: &'a IndexResultViewModel,
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

impl fmt::Display for IndexResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", IndexResultView { data: self })
    }
}

#[derive(Debug, Serialize)]
pub struct VacuumResultViewModel {
    pub success: bool,
}

impl CreateView for VacuumResultViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(VacuumResultView { data: self })
    }
}

struct VacuumResultView<'a> {
    data: &'a VacuumResultViewModel,
}

impl<'a> fmt::Display for VacuumResultView<'a> {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

impl fmt::Display for VacuumResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", VacuumResultView { data: self })
    }
}
