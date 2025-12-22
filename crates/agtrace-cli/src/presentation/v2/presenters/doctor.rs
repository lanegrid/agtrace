use std::collections::HashMap;

use crate::presentation::v2::view_models::{
    CheckStatus, CommandResultViewModel, DiagnoseResultViewModel, DiagnoseResultsViewModel,
    DoctorCheckResultViewModel, FailureExample, Guidance, InspectLine, InspectResultViewModel,
    StatusBadge,
};

pub fn present_diagnose_results(
    results: Vec<agtrace_engine::DiagnoseResult>,
) -> CommandResultViewModel<DiagnoseResultsViewModel> {
    let mut total_files = 0;
    let mut total_successful = 0;
    let mut total_failed = 0;

    let vms: Vec<DiagnoseResultViewModel> = results
        .into_iter()
        .map(|r| {
            total_files += r.total_files;
            total_successful += r.successful;
            let failed_count: usize = r.failures.values().map(|v| v.len()).sum();
            total_failed += failed_count;

            let failure_map: HashMap<String, Vec<FailureExample>> = r
                .failures
                .into_iter()
                .map(|(failure_type, examples)| {
                    let reason = format!("{:?}", failure_type);
                    let examples: Vec<FailureExample> = examples
                        .into_iter()
                        .map(|ex| FailureExample {
                            path: ex.path,
                            reason: ex.reason,
                        })
                        .collect();
                    (reason, examples)
                })
                .collect();

            DiagnoseResultViewModel {
                provider_name: r.provider_name,
                total_files: r.total_files,
                successful: r.successful,
                failures: failure_map,
            }
        })
        .collect();

    let content = DiagnoseResultsViewModel { results: vms };

    let mut result = CommandResultViewModel::new(content);

    if total_failed == 0 {
        result = result
            .with_badge(StatusBadge::success(format!(
                "All {} files parsed successfully",
                total_files
            )))
            .with_suggestion(Guidance::new("Your log files are healthy and parseable"));
    } else {
        result = result
            .with_badge(StatusBadge::warning(format!(
                "{} files failed, {} succeeded",
                total_failed, total_successful
            )))
            .with_suggestion(
                Guidance::new("Use doctor check to validate specific files")
                    .with_command("agtrace doctor check <file-path>"),
            )
            .with_suggestion(
                Guidance::new("Use doctor inspect to view raw file contents")
                    .with_command("agtrace doctor inspect <file-path>"),
            );
    }

    result
}

pub fn present_check_result(
    file_path: String,
    provider_name: String,
    status: agtrace_runtime::CheckStatus,
    event_count: usize,
    error_message: Option<String>,
) -> CommandResultViewModel<DoctorCheckResultViewModel> {
    let check_status = match status {
        agtrace_runtime::CheckStatus::Success => CheckStatus::Success,
        agtrace_runtime::CheckStatus::Failure => CheckStatus::Failure,
    };

    let content = DoctorCheckResultViewModel {
        file_path: file_path.clone(),
        provider_name,
        status: check_status.clone(),
        event_count,
        error_message: error_message.clone(),
    };

    let mut result = CommandResultViewModel::new(content);

    match check_status {
        CheckStatus::Success => {
            result = result
                .with_badge(StatusBadge::success("File is valid"))
                .with_suggestion(Guidance::new(format!(
                    "Successfully parsed {} events",
                    event_count
                )));
        }
        CheckStatus::Failure => {
            result = result
                .with_badge(StatusBadge::error("File validation failed"))
                .with_suggestion(
                    Guidance::new("Inspect the raw file contents")
                        .with_command(format!("agtrace doctor inspect {}", file_path)),
                );

            if let Some(err) = &error_message {
                result = result.with_suggestion(Guidance::new(format!("Error: {}", err)));
            }
        }
    }

    result
}

pub fn present_inspect_result(
    file_path: String,
    total_lines: usize,
    shown_lines: usize,
    lines: Vec<(usize, String)>,
) -> CommandResultViewModel<InspectResultViewModel> {
    let inspect_lines: Vec<InspectLine> = lines
        .into_iter()
        .map(|(number, content)| InspectLine { number, content })
        .collect();

    let content = InspectResultViewModel {
        file_path: file_path.clone(),
        total_lines,
        shown_lines,
        lines: inspect_lines,
    };

    let mut result = CommandResultViewModel::new(content);

    result = result.with_badge(StatusBadge::info(format!(
        "Showing {} of {} lines",
        shown_lines, total_lines
    )));

    if shown_lines < total_lines {
        result = result.with_suggestion(
            Guidance::new("Use --lines to see more content")
                .with_command(format!("agtrace doctor inspect {} --lines 100", file_path)),
        );
    }

    result = result.with_suggestion(
        Guidance::new("Use --format json to parse as JSON").with_command(format!(
            "agtrace doctor inspect {} --format json",
            file_path
        )),
    );

    result
}
