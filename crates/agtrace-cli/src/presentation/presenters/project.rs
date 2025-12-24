use crate::presentation::view_models::{
    CommandResultViewModel, Guidance, ProjectEntryViewModel, ProjectListViewModel, StatusBadge,
};
use agtrace_runtime::ProjectInfo;

pub fn present_project_list(
    current_root: String,
    current_hash: String,
    projects: Vec<ProjectInfo>,
) -> CommandResultViewModel<ProjectListViewModel> {
    let project_entries: Vec<ProjectEntryViewModel> = projects
        .into_iter()
        .map(|p| {
            let hash_short = if p.hash.chars().count() > 16 {
                let truncated: String = p.hash.chars().take(16).collect();
                format!("{}...", truncated)
            } else {
                p.hash.clone()
            };

            ProjectEntryViewModel {
                hash: p.hash,
                hash_short,
                root_path: p.root_path,
                session_count: p.session_count,
                last_scanned: p.last_scanned,
            }
        })
        .collect();

    let content = ProjectListViewModel {
        current_root,
        current_hash,
        projects: project_entries,
    };

    let mut result = CommandResultViewModel::new(content);

    if result.content.projects.is_empty() {
        result = result
            .with_badge(StatusBadge::info("No projects registered"))
            .with_suggestion(
                Guidance::new("Scan for sessions to register projects")
                    .with_command("agtrace index update"),
            );
    } else {
        result = result.with_badge(StatusBadge::success("Projects found"));
    }

    result
}
