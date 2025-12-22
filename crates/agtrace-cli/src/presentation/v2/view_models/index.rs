use serde::Serialize;

use crate::presentation::v2::renderers::ConsolePresentable;

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

impl ConsolePresentable for IndexResultViewModel {
    fn render_console(&self) {
        // Progress events are already printed during scanning
        // This is the final summary
        match self.mode {
            IndexMode::Update => {
                if self.total_sessions > 0 {
                    println!(
                        "\nIndexed {} session(s) ({} files scanned, {} skipped)",
                        self.total_sessions, self.scanned_files, self.skipped_files
                    );
                } else {
                    println!("\nNo new sessions found.");
                }
            }
            IndexMode::Rebuild => {
                println!(
                    "\nRebuilt index: {} session(s) ({} files scanned)",
                    self.total_sessions, self.scanned_files
                );
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct VacuumResultViewModel {
    pub success: bool,
}

impl ConsolePresentable for VacuumResultViewModel {
    fn render_console(&self) {
        // Badge already shows "Database optimized", no need to print duplicate message
    }
}
