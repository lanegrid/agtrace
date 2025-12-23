use crate::presentation::v2::view_models::{ConfigStatus, InitResultViewModel, ProviderInfo, ScanOutcome};

pub fn present_init_result(result: agtrace_runtime::InitResult) -> InitResultViewModel {
    let config_status = match result.config_status {
        agtrace_runtime::ConfigStatus::DetectedAndSaved { providers } => {
            ConfigStatus::DetectedAndSaved {
                providers: providers
                    .into_iter()
                    .map(|(k, v)| (k, v.display().to_string()))
                    .collect(),
            }
        }
        agtrace_runtime::ConfigStatus::LoadedExisting { config_path } => {
            ConfigStatus::LoadedExisting {
                config_path: config_path.display().to_string(),
            }
        }
        agtrace_runtime::ConfigStatus::NoProvidersDetected {
            available_providers,
        } => ConfigStatus::NoProvidersDetected {
            available_providers: available_providers
                .into_iter()
                .map(|p| ProviderInfo {
                    name: p.name,
                    default_log_path: p.default_log_path,
                })
                .collect(),
        },
    };

    let scan_outcome = match result.scan_outcome {
        agtrace_runtime::ScanOutcome::Scanned => ScanOutcome::Scanned,
        agtrace_runtime::ScanOutcome::Skipped { elapsed } => ScanOutcome::Skipped {
            elapsed_seconds: elapsed.num_seconds(),
        },
    };

    InitResultViewModel {
        config_status,
        db_path: result.db_path.display().to_string(),
        scan_outcome,
        session_count: result.session_count,
        all_projects: result.all_projects,
        scan_needed: result.scan_needed,
    }
}
