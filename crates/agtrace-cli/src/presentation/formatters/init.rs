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
            println!("  Completed");
        }
        ScanOutcome::Skipped { elapsed } => {
            println!("  Skipped (scanned {})", format_duration(*elapsed));
            println!("  Use `agtrace init --refresh` to force re-scan.");
        }
    }

    println!("\nRecent sessions:");
    if result.recent_sessions.is_empty() {
        if result.all_projects {
            println!("  No sessions found.");
            println!("\nTips:");
            println!("  - Check provider configuration: agtrace provider list");
            println!("  - Run diagnostics: agtrace doctor run");
        } else {
            println!("  No sessions found for the current project.");
            println!("\nTips:");
            println!("  - Scan all projects: agtrace init --all-projects");
            println!("  - Or: agtrace index update --all-projects");
        }
    } else if let Some(first_session) = result.recent_sessions.first() {
        let session_prefix = if first_session.id.len() > 8 {
            &first_session.id[..8]
        } else {
            &first_session.id
        };

        println!("\nNext steps:");
        println!("  View session in compact style (see bottlenecks and tool chains):");
        println!(
            "    agtrace session show {} --style compact",
            session_prefix
        );
        println!("\n  View conversation only (for LLM consumption):");
        println!(
            "    agtrace session show {} --only user,assistant --full",
            session_prefix
        );
    }
}
