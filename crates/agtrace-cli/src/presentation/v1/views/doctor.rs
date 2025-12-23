use crate::presentation::v1::view_models::{
    DiagnoseResultViewModel, DoctorCheckResultViewModel, DoctorCheckStatus, EventPayloadViewModel,
    EventViewModel,
};
use owo_colors::OwoColorize;
use std::collections::HashMap;

pub fn print_check_result(
    file_path: &str,
    provider_name: &str,
    result: Result<&[EventViewModel], &anyhow::Error>,
) {
    println!("File: {}", file_path);
    println!("Provider: {}", provider_name);

    match result {
        Ok(events) => {
            let first_event = events.first();
            let session_id = first_event
                .map(|e| e.session_id.clone())
                .unwrap_or_default();
            let timestamp = first_event
                .map(|e| e.timestamp)
                .unwrap_or_else(chrono::Utc::now);

            let mut event_breakdown = HashMap::new();
            for event in events {
                let payload_type = match &event.payload {
                    EventPayloadViewModel::User { .. } => "User",
                    EventPayloadViewModel::Message { .. } => "Message",
                    EventPayloadViewModel::ToolCall { .. } => "ToolCall",
                    EventPayloadViewModel::ToolResult { .. } => "ToolResult",
                    EventPayloadViewModel::Reasoning { .. } => "Reasoning",
                    EventPayloadViewModel::TokenUsage { .. } => "TokenUsage",
                    EventPayloadViewModel::Notification { .. } => "Notification",
                };
                *event_breakdown.entry(payload_type).or_insert(0) += 1;
            }

            println!("Status: {}", "✓ Valid".green().bold());
            println!();
            println!("Parsed successfully:");
            println!("  - Session ID: {}", session_id);
            println!("  - Timestamp: {}", timestamp);
            println!("  - Events extracted: {}", events.len());

            if !event_breakdown.is_empty() {
                println!("  - Event breakdown:");
                for (payload_type, count) in event_breakdown {
                    println!("      {}: {}", payload_type, count);
                }
            }
        }
        Err(error) => {
            let error_message = format!("{:#}", error);
            let suggestion = generate_suggestion(&error_message, file_path);

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
            println!("       agtrace inspect {} --lines 20", file_path);
            println!("  2. Compare with expected schema:");
            let provider_first_word = provider_name.split_whitespace().next().unwrap_or("");
            println!("       agtrace schema {}", provider_first_word);
            println!("  3. Update schema definition if needed");
        }
    }
}

pub fn print_check_result_vm(result: &DoctorCheckResultViewModel) {
    println!("File: {}", result.file_path);
    println!("Provider: {}", result.provider_name);

    match result.status {
        DoctorCheckStatus::Success => {
            let first_event = result.events.first();
            let session_id = first_event
                .map(|e| e.session_id.clone())
                .unwrap_or_default();
            let timestamp = first_event
                .map(|e| e.timestamp)
                .unwrap_or_else(chrono::Utc::now);

            let mut event_breakdown = HashMap::new();
            for event in &result.events {
                let payload_type = match &event.payload {
                    EventPayloadViewModel::User { .. } => "User",
                    EventPayloadViewModel::Message { .. } => "Message",
                    EventPayloadViewModel::ToolCall { .. } => "ToolCall",
                    EventPayloadViewModel::ToolResult { .. } => "ToolResult",
                    EventPayloadViewModel::Reasoning { .. } => "Reasoning",
                    EventPayloadViewModel::TokenUsage { .. } => "TokenUsage",
                    EventPayloadViewModel::Notification { .. } => "Notification",
                };
                *event_breakdown.entry(payload_type).or_insert(0) += 1;
            }

            println!("Status: {}", "✓ Valid".green().bold());
            println!();
            println!("Parsed successfully:");
            println!("  - Session ID: {}", session_id);
            println!("  - Timestamp: {}", timestamp);
            println!("  - Events extracted: {}", result.events.len());

            if !event_breakdown.is_empty() {
                println!("  - Event breakdown:");
                for (payload_type, count) in event_breakdown {
                    println!("      {}: {}", payload_type, count);
                }
            }
        }
        DoctorCheckStatus::Failure => {
            let error_message = result.error_message.as_deref().unwrap_or("Unknown error");
            let suggestion = generate_suggestion(error_message, &result.file_path);

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
            println!("       agtrace inspect {} --lines 20", result.file_path);
            println!("  2. Compare with expected schema:");
            let provider_first_word = result.provider_name.split_whitespace().next().unwrap_or("");
            println!("       agtrace schema {}", provider_first_word);
            println!("  3. Update schema definition if needed");
        }
    }
}

fn generate_suggestion(error_msg: &str, file_path: &str) -> Option<String> {
    if error_msg.contains("missing field") {
        Some(
            "This field may have been added in a newer version of the provider.\n\
             Check if the schema definition needs to make this field optional."
                .to_string(),
        )
    } else if error_msg.contains("invalid type") {
        Some(format!(
            "The field type in the schema may not match the actual data format.\n\
             Use 'agtrace inspect {}' to examine the actual structure.\n\
             Use 'agtrace schema <provider>' to see the expected format.",
            file_path
        ))
    } else if error_msg.contains("expected") {
        Some(
            "The data format may have changed between provider versions.\n\
             Consider using an enum or untagged union to support multiple formats."
                .to_string(),
        )
    } else {
        None
    }
}

pub fn print_results(results: &[DiagnoseResultViewModel], verbose: bool) {
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
