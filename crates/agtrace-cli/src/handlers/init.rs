use crate::presentation::renderers::TraceView;
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
        super::index::handle(
            &workspace,
            project_root.as_deref(),
            all_projects,
            "all".to_string(),
            false,
            true,
            view,
        )?;
    }

    Ok(())
}
