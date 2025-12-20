use crate::context::ExecutionContext;
use crate::presentation::renderers::TraceView;
use crate::types::OutputFormat;
use agtrace_index::Database;
use agtrace_runtime::{InitConfig, InitProgress, InitService};
use anyhow::Result;

pub fn handle(ctx: &ExecutionContext, refresh: bool, view: &dyn TraceView) -> Result<()> {
    let config = InitConfig {
        data_dir: ctx.data_dir().to_path_buf(),
        project_root: ctx.project_root.clone(),
        all_projects: ctx.all_projects,
        refresh,
    };

    let result = InitService::run(
        config,
        Some(|progress: InitProgress| {
            let _ = view.render_init_progress(&progress);
        }),
    )?;

    if !result.recent_sessions.is_empty() {
        let db_path = ctx.data_dir().join("agtrace.db");
        let db = Database::open(&db_path)?;
        let effective_hash = if result.all_projects {
            None
        } else {
            let current_project_root = ctx
                .project_root
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| ".".to_string());
            Some(agtrace_types::project_hash_from_root(&current_project_root))
        };

        super::session_list::handle(
            &db,
            effective_hash,
            10,
            result.all_projects,
            OutputFormat::Plain,
            None,
            None,
            None,
            true,
            ctx.data_dir(),
            ctx.project_root.as_ref().map(|p| p.display().to_string()),
            view,
        )?;
    }

    view.render_init_result(&result)?;

    if result.scan_needed {
        super::index::handle(ctx, "all".to_string(), false, true, view)?;
    }

    Ok(())
}
