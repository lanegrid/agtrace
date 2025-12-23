use crate::presentation::v1::view_models::{DiagnoseResultViewModel, FailureExampleViewModel};
use agtrace_engine::DiagnoseResult;

pub fn present_diagnose_result(result: &DiagnoseResult) -> DiagnoseResultViewModel {
    DiagnoseResultViewModel {
        provider_name: result.provider_name.clone(),
        total_files: result.total_files,
        successful: result.successful,
        failures: result
            .failures
            .iter()
            .map(|(k, v)| {
                (
                    format!("{:?}", k),
                    v.iter()
                        .map(|example| FailureExampleViewModel {
                            path: example.path.clone(),
                            reason: example.reason.clone(),
                        })
                        .collect(),
                )
            })
            .collect(),
    }
}

pub fn present_diagnose_results(results: &[DiagnoseResult]) -> Vec<DiagnoseResultViewModel> {
    results.iter().map(present_diagnose_result).collect()
}
