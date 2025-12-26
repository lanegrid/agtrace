use crate::args::{OutputFormat, ViewModeArgs};
use crate::presentation::presenters;
use crate::presentation::view_models::{CommandResultViewModel, InitProgress};
use crate::presentation::views::init::print_init_progress;
use crate::presentation::{ConsoleRenderer, Renderer};
use agtrace_runtime::{AgTrace, InitConfig};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn handle(
    data_dir: &Path,
    project_root: Option<PathBuf>,
    all_projects: bool,
    refresh: bool,
    output_format: OutputFormat,
    view_mode_args: &ViewModeArgs,
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
            print_init_progress(&progress);
        }),
    )?;

    let vm = presenters::present_init_result(result.clone());
    let result_vm = CommandResultViewModel::new(vm);
    let resolved_view_mode = view_mode_args.resolve();
    let renderer = ConsoleRenderer::new(output_format.into(), resolved_view_mode);
    renderer.render(result_vm)?;

    // Provide helpful guidance based on session count
    if result.scan_needed {
        if result.session_count == 0 {
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
