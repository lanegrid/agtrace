use crate::presentation::view_models::{ConfigStatus, InitProgress, InitResult, ScanOutcome};
use chrono::Duration;

fn format_duration(d: Duration) -> String {
    let seconds = d.num_seconds();
    let minutes = d.num_minutes();
    if seconds < 60 {
        format!("{}s ago", seconds)
    } else if minutes < 60 {
        format!("{}m ago", minutes)
    } else {
        let hours = d.num_hours();
        format!("{}h ago", hours)
    }
}

pub fn print_init_progress(progress: &InitProgress) {
    match progress {
        InitProgress::ConfigPhase => println!("Step 1/4: Configuration..."),
        InitProgress::DatabasePhase => println!("Step 2/4: Database..."),
        InitProgress::ScanPhase => println!("Step 3/4: Scanning..."),
        InitProgress::SessionPhase => println!("Step 4/4: Sessions..."),
    }
}

pub fn print_init_result(result: &InitResult) {
    println!("Initializing agtrace...\n");

    match &result.config_status {
        ConfigStatus::DetectedAndSaved { providers } => {
            println!("Configuration:");
            println!("  Detected {} provider(s):", providers.len());
            for (name, log_root) in providers {
                println!("    {} -> {}", name, log_root.display());
            }
            println!("  Configuration saved");
        }
        ConfigStatus::LoadedExisting { config_path } => {
            println!("Configuration:");
            println!("  Loaded from {}", config_path.display());
        }
        ConfigStatus::NoProvidersDetected {
            available_providers,
        } => {
            println!("Configuration:");
            println!("  No providers detected automatically.");
            println!("\n  To manually configure a provider:");
            println!("    agtrace provider set <name> --log-root <PATH> --enable");
            println!("\n  Supported providers:");
            for provider in available_providers {
                println!(
                    "    - {}  (default: {})",
                    provider.name, provider.default_log_path
                );
            }
            return;
        }
    }

    println!("\nDatabase:");
    println!("  Ready at {}", result.db_path.display());

    println!("\nScan:");
    match &result.scan_outcome {
        ScanOutcome::Scanned => {
            if result.scan_needed {
                println!("  Scanning logs...");
            } else {
                println!("  Completed");
            }
        }
        ScanOutcome::Skipped { elapsed } => {
            println!("  Skipped (scanned {})", format_duration(*elapsed));
            println!("  Use `agtrace init --refresh` to force re-scan.");
        }
    }

    if !result.scan_needed {
        println!("\nSessions:");
        if result.session_count == 0 {
            if result.all_projects {
                println!("  No sessions found in global index.");
                println!("\nTips:");
                println!("  - Check provider configuration: agtrace provider list");
                println!("  - Run diagnostics: agtrace doctor run");
            } else {
                println!("  Current directory: No sessions linked to this project.");
                println!("\nTips:");
                println!("  - To see all indexed sessions: agtrace list --all-projects");
                println!("  - To scan all projects: agtrace init --all-projects");
            }
        } else {
            if result.all_projects {
                println!(
                    "  Found {} sessions across all projects",
                    result.session_count
                );
            } else {
                println!(
                    "  Found {} sessions for current project",
                    result.session_count
                );
            }
            println!("\nNext steps:");
            println!("  View recent sessions:");
            println!("    agtrace list");
            println!("\n  View specific session:");
            println!("    agtrace session show <id> --style compact");
        }
    }
}
