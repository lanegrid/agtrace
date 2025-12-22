use serde::Serialize;
use std::fmt;

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

impl fmt::Display for IndexResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Progress events are already printed during scanning
        // This is the final summary
        match self.mode {
            IndexMode::Update => {
                if self.total_sessions > 0 {
                    writeln!(
                        f,
                        "\nIndexed {} session(s) ({} files scanned, {} skipped)",
                        self.total_sessions, self.scanned_files, self.skipped_files
                    )?;
                } else {
                    writeln!(f, "\nNo new sessions found.")?;
                }
            }
            IndexMode::Rebuild => {
                writeln!(
                    f,
                    "\nRebuilt index: {} session(s) ({} files scanned)",
                    self.total_sessions, self.scanned_files
                )?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct VacuumResultViewModel {
    pub success: bool,
}

impl fmt::Display for VacuumResultViewModel {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        // Badge already shows "Database optimized", no need to print duplicate message
        Ok(())
    }
}
