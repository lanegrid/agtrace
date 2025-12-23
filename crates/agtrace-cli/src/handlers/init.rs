use crate::presentation::v1::renderers::TraceView;
use agtrace_runtime::{AgTrace, InitConfig, InitProgress};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn handle(
    data_dir: &Path,
    project_root: Option<PathBuf>,
    all_projects: bool,
    refresh: bool,
    view: &dyn TraceView,
) -> Result<()> {
    let config = InitConfig {
        data_dir: data_dir.to_path_buf(),
        project_root: project_root.clone(),
        all_projects,
        refresh,
    };

    let result = AgTrace::setup(
        config,
        Some(|progress: InitProgress| {
            let _ = view.render_init_progress(&progress);
        }),
    )?;

    view.render_init_result(&result)?;

    if result.scan_needed {
        // Open workspace after init to run index
        let workspace = AgTrace::open(data_dir.to_path_buf())?;
        let default_view_mode = crate::args::ViewModeArgs {
            quiet: false,
            compact: false,
            verbose: false,
        };
        super::index::handle_v2(
            &workspace,
            project_root.as_deref(),
            all_projects,
            "all".to_string(),
            false,
            false,
            crate::args::OutputFormat::Plain,
            &default_view_mode,
        )?;

        // Check session count to provide helpful guidance
        let current_project_root = project_root.as_ref().map(|p| p.display().to_string());
        let current_project_hash = if let Some(root) = &current_project_root {
            agtrace_types::project_hash_from_root(root)
        } else {
            "unknown".to_string()
        };

        let effective_hash = if all_projects {
            None
        } else {
            Some(current_project_hash.as_str())
        };

        let db = workspace.database();
        let sessions = db.lock().unwrap().list_sessions(effective_hash, 10)?;

        if sessions.is_empty() {
            println!();
            if all_projects {
                println!("No sessions found in global index.");
                println!("\nTips:");
                println!("  - Check provider configuration: agtrace provider list");
                println!("  - Run diagnostics: agtrace doctor run");
            } else {
                println!("Current directory: No sessions linked to this project.");
                println!("\nTips:");
                println!("  - To see all indexed sessions: agtrace list --all-projects");
                println!("  - To scan all projects: agtrace init --all-projects");
            }
        } else {
            println!("\nDone! Use 'agtrace list' to see all sessions.");
        }
    }

    Ok(())
}
