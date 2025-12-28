use crate::args::hints::cmd;
use crate::presentation::view_models::{
    CommandResultViewModel, Guidance, IndexMode, IndexResultViewModel, StatusBadge,
    VacuumResultViewModel,
};

pub fn present_index_result(
    total_sessions: usize,
    scanned_files: usize,
    skipped_files: usize,
    is_rebuild: bool,
) -> CommandResultViewModel<IndexResultViewModel> {
    let mode = if is_rebuild {
        IndexMode::Rebuild
    } else {
        IndexMode::Update
    };

    let content = IndexResultViewModel {
        total_sessions,
        scanned_files,
        skipped_files,
        mode,
    };

    let mut result = CommandResultViewModel::new(content);

    if total_sessions == 0 {
        result = result
            .with_badge(StatusBadge::info("No sessions found"))
            .with_suggestion(
                Guidance::new("Check if providers are configured correctly")
                    .with_command(cmd::PROVIDER_LIST),
            )
            .with_suggestion(
                Guidance::new("Run diagnostics to identify issues").with_command(cmd::DOCTOR_RUN),
            );
    } else {
        let label = if is_rebuild {
            format!("Rebuilt {} session(s)", total_sessions)
        } else {
            format!("Indexed {} session(s)", total_sessions)
        };

        result = result.with_badge(StatusBadge::success(label));

        if total_sessions > 0 {
            result = result.with_suggestion(
                Guidance::new("View recent sessions").with_command(cmd::SESSION_LIST),
            );
        }

        if skipped_files > 0 && !is_rebuild {
            result = result.with_suggestion(
                Guidance::new(format!(
                    "Skipped {} files. Use --verbose to see details or rebuild to force rescan",
                    skipped_files
                ))
                .with_command(cmd::INDEX_REBUILD),
            );
        }
    }

    result
}

pub fn present_vacuum_result() -> CommandResultViewModel<VacuumResultViewModel> {
    let content = VacuumResultViewModel { success: true };

    CommandResultViewModel::new(content)
        .with_badge(StatusBadge::success("Database optimized"))
        .with_suggestion(Guidance::new(
            "Vacuum reclaims unused space and optimizes query performance",
        ))
}
