use clap::Args;

#[derive(Debug, Clone, Default, Args)]
pub struct ViewModeArgs {
    #[arg(
        long,
        help_heading = "Output Options",
        help = "Minimal output (IDs only, for scripting)",
        group = "view_mode"
    )]
    pub quiet: bool,

    #[arg(
        long,
        help_heading = "Output Options",
        help = "Compact output (one line per item)",
        group = "view_mode"
    )]
    pub compact: bool,

    #[arg(
        long,
        help_heading = "Output Options",
        help = "Verbose output (all metadata)",
        group = "view_mode"
    )]
    pub verbose: bool,
}

impl ViewModeArgs {
    pub fn resolve(&self) -> crate::presentation::ViewMode {
        use crate::presentation::ViewMode;

        if self.quiet {
            ViewMode::Minimal
        } else if self.compact {
            ViewMode::Compact
        } else if self.verbose {
            ViewMode::Verbose
        } else {
            ViewMode::default()
        }
    }
}
