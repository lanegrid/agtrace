use serde::Serialize;
use std::collections::HashMap;

// Re-export progress type (not serialized, used for ephemeral console output)
pub use agtrace_runtime::InitProgress;

#[derive(Debug, Clone, Serialize)]
pub struct ProviderInfo {
    pub name: String,
    pub default_log_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ConfigStatus {
    DetectedAndSaved {
        providers: HashMap<String, String>,
    },
    LoadedExisting {
        config_path: String,
    },
    NoProvidersDetected {
        available_providers: Vec<ProviderInfo>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ScanOutcome {
    Scanned,
    Skipped { elapsed_seconds: i64 },
}

#[derive(Debug, Clone, Serialize)]
pub struct InitResultViewModel {
    pub config_status: ConfigStatus,
    pub db_path: String,
    pub scan_outcome: ScanOutcome,
    pub session_count: usize,
    pub all_projects: bool,
    pub scan_needed: bool,
}
