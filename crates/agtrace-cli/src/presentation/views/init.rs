use std::fmt;

use crate::args::hints::cmd;
use crate::presentation::view_models::{
    ConfigStatus, CreateView, InitResultViewModel, ScanOutcome, ViewMode,
};

impl CreateView for InitResultViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(InitResultView { data: self })
    }
}

struct InitResultView<'a> {
    data: &'a InitResultViewModel,
}

impl<'a> InitResultView<'a> {
    fn format_duration_seconds(&self, seconds: i64) -> String {
        let minutes = seconds / 60;
        if seconds < 60 {
            format!("{}s ago", seconds)
        } else if minutes < 60 {
            format!("{}m ago", minutes)
        } else {
            let hours = minutes / 60;
            format!("{}h ago", hours)
        }
    }
}

impl<'a> fmt::Display for InitResultView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Initializing agtrace...\n")?;

        match &self.data.config_status {
            ConfigStatus::DetectedAndSaved { providers } => {
                writeln!(f, "Configuration:")?;
                writeln!(f, "  Detected {} provider(s):", providers.len())?;
                for (name, log_root) in providers {
                    writeln!(f, "    {} -> {}", name, log_root)?;
                }
                writeln!(f, "  Configuration saved")?;
            }
            ConfigStatus::LoadedExisting { config_path } => {
                writeln!(f, "Configuration:")?;
                writeln!(f, "  Loaded from {}", config_path)?;
            }
            ConfigStatus::NoProvidersDetected {
                available_providers,
            } => {
                writeln!(f, "Configuration:")?;
                writeln!(f, "  No providers detected automatically.")?;
                writeln!(f, "\n  To manually configure a provider:")?;
                writeln!(f, "    {}", cmd::PROVIDER_SET)?;
                writeln!(f, "\n  Supported providers:")?;
                for provider in available_providers {
                    writeln!(
                        f,
                        "    - {}  (default: {})",
                        provider.name, provider.default_log_path
                    )?;
                }
                return Ok(());
            }
        }

        writeln!(f, "\nDatabase:")?;
        writeln!(f, "  Ready at {}", self.data.db_path)?;

        writeln!(f, "\nScan:")?;
        match &self.data.scan_outcome {
            ScanOutcome::Scanned => {
                if self.data.scan_needed {
                    writeln!(f, "  Completed scanning")?;
                } else {
                    writeln!(f, "  Completed")?;
                }
            }
            ScanOutcome::Skipped { elapsed_seconds } => {
                writeln!(
                    f,
                    "  Skipped (scanned {})",
                    self.format_duration_seconds(*elapsed_seconds)
                )?;
                writeln!(f, "  Use `{}` to force re-scan.", cmd::INIT_REFRESH)?;
            }
        }

        writeln!(f, "\nSessions:")?;
        if self.data.session_count == 0 {
            if self.data.all_projects {
                writeln!(f, "  No sessions found in global index.")?;
                writeln!(f, "\nTips:")?;
                writeln!(
                    f,
                    "  - Check provider configuration: {}",
                    cmd::PROVIDER_LIST
                )?;
                writeln!(f, "  - Run diagnostics: {}", cmd::DOCTOR_RUN)?;
            } else {
                writeln!(
                    f,
                    "  Current directory: No sessions linked to this project."
                )?;
                writeln!(f, "\nTips:")?;
                writeln!(
                    f,
                    "  - To see all indexed sessions: {}",
                    cmd::LIST_ALL_PROJECTS
                )?;
                writeln!(f, "  - To scan all projects: {}", cmd::INIT_ALL_PROJECTS)?;
            }
        } else {
            if self.data.all_projects {
                writeln!(
                    f,
                    "  Found {} sessions across all projects",
                    self.data.session_count
                )?;
            } else {
                writeln!(
                    f,
                    "  Found {} sessions for current project",
                    self.data.session_count
                )?;
            }
            writeln!(f, "\nNext steps:")?;
            writeln!(f, "  View recent sessions:")?;
            writeln!(f, "    {}", cmd::LIST)?;
            writeln!(f, "\n  View specific session:")?;
            writeln!(f, "    {}", cmd::SESSION_SHOW_COMPACT)?;
        }

        Ok(())
    }
}

/// Helper function to print init progress (not part of ViewModel/View pattern)
/// Progress messages are ephemeral and not meant for JSON output
pub fn print_init_progress(progress: &crate::presentation::view_models::InitProgress) {
    use crate::presentation::view_models::InitProgress;
    match progress {
        InitProgress::ConfigPhase => println!("Step 1/4: Configuration..."),
        InitProgress::DatabasePhase => println!("Step 2/4: Database..."),
        InitProgress::ScanPhase => println!("Step 3/4: Scanning..."),
        InitProgress::SessionPhase => println!("Step 4/4: Sessions..."),
    }
}
