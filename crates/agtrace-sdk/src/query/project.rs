//! Project query types.

use serde::Serialize;

use crate::types::ProjectInfo;

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct ProjectInfoViewModel(pub Vec<ProjectInfo>);

impl ProjectInfoViewModel {
    pub fn new(projects: Vec<ProjectInfo>) -> Self {
        Self(projects)
    }
}
