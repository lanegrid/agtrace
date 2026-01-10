use crate::args::hints::cmd;
use crate::presentation::view_models::{
    CommandResultViewModel, Guidance, IndexInfoViewModel, IndexMode, IndexResultViewModel,
    StatusBadge, VacuumResultViewModel,
};
use std::path::PathBuf;

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

pub fn present_index_info(
    data_dir: PathBuf,
    db_path: PathBuf,
    config_path: PathBuf,
    db_exists: bool,
    db_size_bytes: u64,
) -> CommandResultViewModel<IndexInfoViewModel> {
    let content = IndexInfoViewModel {
        data_dir,
        db_path,
        config_path,
        db_exists,
        db_size_bytes,
    };

    let mut result = CommandResultViewModel::new(content);

    if !db_exists {
        result = result
            .with_badge(StatusBadge::warning("Database not found"))
            .with_suggestion(
                Guidance::new("Initialize the database first").with_command(cmd::INIT),
            );
    } else {
        result = result.with_badge(StatusBadge::success("Database ready"));
    }

    result
}
