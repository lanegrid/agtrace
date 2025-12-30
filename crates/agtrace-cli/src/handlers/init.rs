use crate::args::{OutputFormat, ViewModeArgs};
use crate::presentation::presenters;
use crate::presentation::view_models::{
    CommandResultViewModel, InitProgress as InitProgressViewModel,
};
use crate::presentation::views::init::print_init_progress;
use crate::presentation::{ConsoleRenderer, Renderer};
use agtrace_sdk::SystemClient;
use agtrace_sdk::types::InitConfig;
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

    let result = SystemClient::initialize(
        config,
        Some(|progress: agtrace_sdk::types::InitProgress| {
            let vm: InitProgressViewModel = progress;
            print_init_progress(&vm);
        }),
    )?;

    let vm = presenters::present_init_result(result);
    let result_vm = CommandResultViewModel::new(vm);
    let resolved_view_mode = view_mode_args.resolve();
    let renderer = ConsoleRenderer::new(output_format.into(), resolved_view_mode);
    renderer.render(result_vm)?;

    Ok(())
}
