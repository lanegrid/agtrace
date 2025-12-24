use crate::args::{OutputFormat, ViewModeArgs};
use crate::presentation::presenters;
use crate::presentation::view_models::IndexEvent;
use agtrace_providers::ScanContext;
use agtrace_runtime::{AgTrace, IndexProgress};
use agtrace_types::project_hash_from_root;
use anyhow::Result;
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn handle(
    workspace: &AgTrace,
    project_root: Option<&Path>,
    all_projects: bool,
    _provider: String,
    force: bool,
    verbose: bool,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
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

    // Track final result
    let mut final_total = 0;
    let mut final_scanned = 0;
    let mut final_skipped = 0;

    workspace
        .projects()
        .scan(&scan_context, force, |progress| {
            // Don't render progress events in JSON mode
            if format == OutputFormat::Json {
                // Just capture the final stats
                if let IndexProgress::Completed {
                    total_sessions,
                    scanned_files,
                    skipped_files,
                } = progress
                {
                    final_total = total_sessions;
                    final_scanned = scanned_files;
                    final_skipped = skipped_files;
                }
                return;
            }

            // In console mode, render progress events
            let should_render = match &progress {
                IndexProgress::SessionRegistered { .. } => verbose,
                IndexProgress::IncrementalHint { .. } => verbose,
                IndexProgress::Completed { .. } => false, // We'll render this ourselves with the presentation layer
                IndexProgress::ProviderScanning { .. } => true,
                IndexProgress::ProviderSessionCount { .. } => true,
                IndexProgress::LogRootMissing { .. } => true,
            };

            if should_render {
                let event = map_progress_to_view_model(progress.clone(), verbose);
                render_progress_event(&event);
            }

            // Capture final stats
            if let IndexProgress::Completed {
                total_sessions,
                scanned_files,
                skipped_files,
            } = progress
            {
                final_total = total_sessions;
                final_scanned = scanned_files;
                final_skipped = skipped_files;
            }
        })?;

    let view_model =
        presenters::present_index_result(final_total, final_scanned, final_skipped, force);

    let ctx = crate::handlers::HandlerContext::new(format, view_mode);
    ctx.render(view_model)
}

pub fn handle_vacuum(
    workspace: &AgTrace,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    let ctx = crate::handlers::HandlerContext::new(format, view_mode);

    let db = workspace.database();
    let db = db.lock().unwrap();
    db.vacuum()?;

    let view_model = presenters::present_vacuum_result();
    ctx.render(view_model)
}

fn render_progress_event(event: &IndexEvent) {
    use std::io::Write;

    match event {
        IndexEvent::IncrementalHint { indexed_files } => {
            println!(
                "Incremental scan mode: {} files already indexed",
                indexed_files
            );
        }
        IndexEvent::LogRootMissing {
            provider_name,
            log_root,
        } => {
            println!(
                "  [Skip] {}: Log root not found at {}",
                provider_name,
                log_root.display()
            );
        }
        IndexEvent::ProviderScanning { provider_name } => {
            print!("  • {:<15} ", provider_name);
            std::io::stdout().flush().unwrap();
        }
        IndexEvent::ProviderSessionCount {
            provider_name: _,
            count,
            project_hash: _,
            all_projects: _,
        } => {
            println!("Found {} sessions", count);
        }
        IndexEvent::SessionRegistered { session_id } => {
            println!("  Registered: {}", session_id);
        }
        IndexEvent::Completed { .. } => {
            // Handled by the presenter
        }
    }
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
