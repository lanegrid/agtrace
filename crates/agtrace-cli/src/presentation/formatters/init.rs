use chrono::Duration;

// Re-export from view_models
pub use crate::presentation::view_models::{SkipReason, Step1Result, Step3Result};

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

pub fn print_init_header() {
    println!("Initializing agtrace...\n");
}

pub fn print_step1_detecting() {
    println!("Step 1/4: Detecting providers...");
}

pub fn print_step1_loading() {
    println!("Step 1/4: Loading configuration...");
}

pub fn print_step1_result(result: &Step1Result) {
    match result {
        Step1Result::DetectedProviders {
            providers,
            config_saved,
        } => {
            println!("  Detected {} provider(s):", providers.len());
            for (name, log_root) in providers {
                println!("    {} -> {}", name, log_root.display());
            }
            if *config_saved {
                println!("  Configuration saved");
            }
        }
        Step1Result::LoadedConfig { config_path } => {
            println!("  Configuration loaded from {}", config_path.display());
        }
        Step1Result::NoProvidersDetected {
            available_providers,
        } => {
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
        }
    }
}

pub fn print_step2_header() {
    println!("\nStep 2/4: Setting up database...");
}

pub fn print_step2_result(db_path: &std::path::Path) {
    println!("  Database ready at {}", db_path.display());
}

pub fn print_step3_header() {
    println!("\nStep 3/4: Scanning for sessions...");
}

pub fn print_step3_result(result: &Step3Result) {
    match result {
        Step3Result::Scanned { success, error } => {
            if !success {
                if let Some(err_msg) = error {
                    println!("  Warning: Scan completed with errors: {}", err_msg);
                    println!("\n  If you encounter compatibility issues, run:");
                    println!("    agtrace doctor run");
                }
            }
        }
        Step3Result::Skipped { reason } => match reason {
            SkipReason::RecentlyScanned { elapsed } => {
                println!(
                    "  Recently scanned ({}). Skipping.",
                    format_duration(*elapsed)
                );
                println!("  Use `agtrace init --refresh` to force re-scan.");
            }
        },
    }
}

pub fn print_step4_header() {
    println!("\nStep 4/4: Recent sessions...\n");
}

pub fn print_step4_no_sessions(all_projects: bool) {
    if all_projects {
        println!("No sessions found.");
        println!("\nTips:");
        println!("  - Check provider configuration: agtrace provider list");
        println!("  - Run diagnostics: agtrace doctor run");
    } else {
        println!("No sessions found for the current project.");
        println!("\nTips:");
        println!("  - Scan all projects: agtrace init --all-projects");
        println!("  - Or: agtrace index update --all-projects");
    }
}

pub fn print_next_steps(first_session_id: &str) {
    let session_prefix = if first_session_id.len() > 8 {
        &first_session_id[..8]
    } else {
        first_session_id
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
