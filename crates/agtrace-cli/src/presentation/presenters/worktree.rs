use crate::presentation::view_models::{
    CommandResultViewModel, StatusBadge, WorktreeListViewModel, WorktreeSessionsViewModel,
};

pub fn present_worktree_list(
    view_model: WorktreeListViewModel,
) -> CommandResultViewModel<WorktreeListViewModel> {
    let worktree_count = view_model.worktrees.len();

    let mut result = CommandResultViewModel::new(view_model);

    if worktree_count == 0 {
        result = result.with_badge(StatusBadge::info("No worktrees found"));
    } else {
        result = result.with_badge(StatusBadge::success(format!(
            "{} worktree(s)",
            worktree_count
        )));
    }

    result
}

pub fn present_worktree_sessions(
    view_model: WorktreeSessionsViewModel,
) -> CommandResultViewModel<WorktreeSessionsViewModel> {
    let worktree_count = view_model.groups.len();
    let session_count = view_model.total_sessions;

    let mut result = CommandResultViewModel::new(view_model);

    if session_count == 0 {
        result = result.with_badge(StatusBadge::info("No sessions found"));
    } else {
        result = result.with_badge(StatusBadge::success(format!(
            "{} session(s) across {} worktree(s)",
            session_count, worktree_count
        )));
    }

    result
}
