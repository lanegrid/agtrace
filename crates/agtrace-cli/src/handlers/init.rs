use super::context::ExecutionContext;
use crate::presentation::renderers::TraceView;
use agtrace_runtime::{AgTrace, InitConfig, InitProgress};
use anyhow::Result;

pub fn handle(ctx: &ExecutionContext, refresh: bool, view: &dyn TraceView) -> Result<()> {
    let config = InitConfig {
        data_dir: ctx.data_dir().to_path_buf(),
        project_root: ctx.project_root.clone(),
        all_projects: ctx.all_projects,
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
        super::index::handle(ctx, "all".to_string(), false, true, view)?;
    }

    Ok(())
}
