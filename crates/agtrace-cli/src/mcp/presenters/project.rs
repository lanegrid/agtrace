use crate::mcp::models::response::ProjectInfoViewModel;

pub fn present_project_info(
    projects: Vec<agtrace_sdk::types::ProjectInfo>,
) -> ProjectInfoViewModel {
    ProjectInfoViewModel(projects)
}
