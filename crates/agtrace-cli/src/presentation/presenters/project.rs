use crate::args::hints::cmd;
use crate::presentation::view_models::{
    CommandResultViewModel, Guidance, ProjectEntryViewModel, ProjectListViewModel, StatusBadge,
};
use agtrace_sdk::types::ProjectInfo;

pub fn present_project_list(
    current_root: String,
    current_hash: String,
    mut projects: Vec<ProjectInfo>,
) -> CommandResultViewModel<ProjectListViewModel> {
    const DEFAULT_LIMIT: usize = 10;

    let total_count = projects.len();

    // Sort by session count (descending)
    projects.sort_by(|a, b| b.session_count.cmp(&a.session_count));

    // Separate current project and others
    let current_project_idx = projects.iter().position(|p| p.hash == current_hash);
    let current_project = current_project_idx.and_then(|idx| {
        if idx < DEFAULT_LIMIT {
            None // Already in top 10
        } else {
            Some(projects.remove(idx))
        }
    });

    // Take top projects
    let mut limited_projects: Vec<ProjectInfo> = projects.into_iter().take(DEFAULT_LIMIT).collect();

    // Add current project if it wasn't in top 10
    if let Some(current) = current_project {
        limited_projects.insert(0, current);
    }

    let project_entries: Vec<ProjectEntryViewModel> = limited_projects
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
                    .with_command(cmd::INDEX_UPDATE),
            );
    } else {
        let shown_count = result.content.projects.len();
        let badge_text = if total_count > DEFAULT_LIMIT {
            format!("Showing top {} of {} projects", shown_count, total_count)
        } else {
            format!("{} project(s)", total_count)
        };
        result = result.with_badge(StatusBadge::success(&badge_text));

        // Add tip if there are more projects
        if total_count > DEFAULT_LIMIT {
            result = result.with_suggestion(Guidance::new(format!(
                "{} more projects not shown (sorted by session count)",
                total_count - shown_count
            )));
        }
    }

    result
}
