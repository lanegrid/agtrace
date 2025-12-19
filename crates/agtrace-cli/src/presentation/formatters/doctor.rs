use crate::display_model::{CheckResult, DoctorCheckDisplay};
use agtrace_engine::DiagnoseResult;
use owo_colors::OwoColorize;

pub fn print_check_result(display: &DoctorCheckDisplay) {
    println!("File: {}", display.file_path);
    println!("Provider: {}", display.provider_name);

    match &display.result {
        CheckResult::Valid {
            session_id,
            timestamp,
            event_count,
            event_breakdown,
        } => {
            println!("Status: {}", "✓ Valid".green().bold());
            println!();
            println!("Parsed successfully:");
            println!("  - Session ID: {}", session_id);
            println!("  - Timestamp: {}", timestamp);
            println!("  - Events extracted: {}", event_count);

            if !event_breakdown.is_empty() {
                println!("  - Event breakdown:");
                for (payload_type, count) in event_breakdown {
                    println!("      {}: {}", payload_type, count);
                }
            }
        }
        CheckResult::Invalid {
            error_message,
            suggestion,
        } => {
            println!("Status: {}", "✗ Invalid".red().bold());
            println!();
            println!("Parse error:");
            println!("  {}", error_message.red());
            println!();

            if let Some(suggestion_text) = suggestion {
                println!("{}", "Suggestion:".cyan().bold());
                for line in suggestion_text.lines() {
                    println!("  {}", line);
                }
            }

            println!();
            println!("Next steps:");
            println!("  1. Examine the actual data:");
            println!("       agtrace inspect {} --lines 20", display.file_path);
            println!("  2. Compare with expected schema:");
            let provider_first_word = display
                .provider_name
                .split_whitespace()
                .next()
                .unwrap_or("");
            println!("       agtrace schema {}", provider_first_word);
            println!("  3. Update schema definition if needed");
        }
    }
}

pub fn print_results(results: &[DiagnoseResult], verbose: bool) {
    println!("{}", "=== Diagnose Results ===".bold());
    println!();

    let mut total_failures = 0;

    for result in results {
        println!("Provider: {}", result.provider_name.bright_blue().bold());
        println!("  Total files scanned: {}", result.total_files);

        let success_rate = if result.total_files > 0 {
            (result.successful as f64 / result.total_files as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "  Successfully parsed: {} ({:.1}%)",
            result.successful.to_string().green(),
            success_rate
        );

        let failure_count = result.total_files - result.successful;
        if failure_count > 0 {
            println!(
                "  Parse failures: {} ({:.1}%)",
                failure_count.to_string().red(),
                100.0 - success_rate
            );
            println!();
            println!("  Failure breakdown:");

            for (failure_type, examples) in &result.failures {
                println!("  {} {}: {} files", "✗".red(), failure_type, examples.len());

                let display_count = if verbose {
                    examples.len()
                } else {
                    1.min(examples.len())
                };

                for example in examples.iter().take(display_count) {
                    println!("    Example: {}", example.path.bright_black());
                    println!("    Reason: {}", example.reason);
                    println!();
                }

                if !verbose && examples.len() > 1 {
                    println!("    ... and {} more files", examples.len() - 1);
                    println!();
                }
            }

            total_failures += failure_count;
        }

        println!();
    }

    println!("{}", "---".bright_black());
    if total_failures > 0 {
        println!(
            "Summary: {} files need schema updates to parse correctly",
            total_failures.to_string().yellow()
        );
        if !verbose {
            println!(
                "Run with {} to see all problematic files",
                "--verbose".cyan()
            );
        }
    } else {
        println!("{}", "All files parsed successfully!".green().bold());
    }
}
