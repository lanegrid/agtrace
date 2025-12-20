use crate::presentation::renderers::TraceView;
use crate::presentation::view_models::IndexEvent;
use agtrace_providers::ScanContext;
use agtrace_runtime::{AgTrace, IndexProgress};
use agtrace_types::project_hash_from_root;
use anyhow::Result;
use std::path::Path;

pub fn handle(
    workspace: &AgTrace,
    project_root: Option<&Path>,
    all_projects: bool,
    _provider: String,
    force: bool,
    verbose: bool,
    view: &dyn TraceView,
) -> Result<()> {
    let current_project_root = project_root.map(|p| p.display().to_string());

    let project_hash = if let Some(root) = &current_project_root {
        project_hash_from_root(root)
    } else {
        "unknown".to_string()
    };

    let scan_context = ScanContext {
        project_hash,
        project_root: if all_projects {
            None
        } else {
            current_project_root
        },
    };

    workspace
        .projects()
        .scan(&scan_context, force, |progress| {
            if verbose || matches!(progress, IndexProgress::Completed { .. }) {
                let event = map_progress_to_view_model(progress, verbose);
                let _ = view.render_index_event(event);
            }
        })?;

    Ok(())
}

fn map_progress_to_view_model(progress: IndexProgress, verbose: bool) -> IndexEvent {
    match progress {
        IndexProgress::IncrementalHint { indexed_files } => {
            IndexEvent::IncrementalHint { indexed_files }
        }
        IndexProgress::LogRootMissing {
            provider_name,
            log_root,
        } => IndexEvent::LogRootMissing {
            provider_name,
            log_root,
        },
        IndexProgress::ProviderScanning { provider_name } => {
            IndexEvent::ProviderScanning { provider_name }
        }
        IndexProgress::ProviderSessionCount {
            provider_name,
            count,
            project_hash,
            all_projects,
        } => IndexEvent::ProviderSessionCount {
            provider_name,
            count,
            project_hash,
            all_projects,
        },
        IndexProgress::SessionRegistered { session_id } => {
            IndexEvent::SessionRegistered { session_id }
        }
        IndexProgress::Completed {
            total_sessions,
            scanned_files,
            skipped_files,
        } => IndexEvent::Completed {
            total_sessions,
            scanned_files,
            skipped_files,
            verbose,
        },
    }
}
