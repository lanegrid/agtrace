use serde::Serialize;

// No request type for get_project_info (no parameters)

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct ProjectInfoViewModel(pub Vec<agtrace_sdk::types::ProjectInfo>);

impl ProjectInfoViewModel {
    pub fn new(projects: Vec<agtrace_sdk::types::ProjectInfo>) -> Self {
        Self(projects)
    }
}
