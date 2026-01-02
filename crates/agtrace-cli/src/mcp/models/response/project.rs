use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct ProjectInfoViewModel(pub Vec<agtrace_sdk::types::ProjectInfo>);
