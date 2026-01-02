use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct AnalysisViewModel(pub agtrace_sdk::AnalysisReport);
