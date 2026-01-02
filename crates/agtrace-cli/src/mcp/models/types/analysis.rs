use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Run diagnostic analysis on a session to identify failures, loops, and issues
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeSessionArgs {
    /// Session ID obtained from list_sessions response (use the 'id' field).
    /// Accepts 8-character prefix (e.g., "fb3cff44") or full UUID.
    /// REQUIRED: Cannot be empty.
    pub session_id: String,
    /// Include failure analysis (default: true)
    #[serde(default)]
    pub include_failures: Option<bool>,
    /// Include loop detection (default: false)
    #[serde(default)]
    pub include_loops: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct AnalysisViewModel(pub agtrace_sdk::AnalysisReport);

impl AnalysisViewModel {
    pub fn new(report: agtrace_sdk::AnalysisReport) -> Self {
        Self(report)
    }
}
