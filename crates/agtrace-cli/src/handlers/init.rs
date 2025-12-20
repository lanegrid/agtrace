use crate::context::ExecutionContext;
use crate::presentation::renderers::TraceView;
use crate::presentation::view_models::{
    InitRenderEvent, ProviderInfo, SkipReason, Step1Result, Step3Result,
};
use crate::types::OutputFormat;
use agtrace_index::Database;
use agtrace_runtime::{InitConfig, InitEvent, InitService};
use anyhow::Result;

pub fn handle(ctx: &ExecutionContext, refresh: bool, view: &dyn TraceView) -> Result<()> {
    let config = InitConfig {
        data_dir: ctx.data_dir().to_path_buf(),
        project_root: ctx.project_root.clone(),
        all_projects: ctx.all_projects,
        refresh,
    };

    let mut scan_needed = false;

    InitService::run(config, |event| {
        match event {
            InitEvent::Header => {
                view.render_init_event(InitRenderEvent::Header)?;
            }
            InitEvent::Step1Detecting => {
                view.render_init_event(InitRenderEvent::Step1Detecting)?;
            }
            InitEvent::Step1Loading => {
                view.render_init_event(InitRenderEvent::Step1Loading)?;
            }
            InitEvent::Step1DetectedProviders {
                providers,
                config_saved,
            } => {
                let step1_result = Step1Result::DetectedProviders {
                    providers,
                    config_saved,
                };
                view.render_init_event(InitRenderEvent::Step1Result(step1_result))?;
            }
            InitEvent::Step1LoadedConfig { config_path } => {
                let step1_result = Step1Result::LoadedConfig { config_path };
                view.render_init_event(InitRenderEvent::Step1Result(step1_result))?;
            }
            InitEvent::Step1NoProvidersDetected {
                available_providers,
            } => {
                let providers: Vec<_> = available_providers
                    .iter()
                    .map(|p| ProviderInfo {
                        name: p.name.clone(),
                        default_log_path: p.default_log_path.clone(),
                    })
                    .collect();
                let step1_result = Step1Result::NoProvidersDetected {
                    available_providers: providers,
                };
                view.render_init_event(InitRenderEvent::Step1Result(step1_result))?;
            }
            InitEvent::Step2Header => {
                view.render_init_event(InitRenderEvent::Step2Header)?;
            }
            InitEvent::Step2DbReady { db_path } => {
                view.render_init_event(InitRenderEvent::Step2Result { db_path })?;
            }
            InitEvent::Step3Header => {
                view.render_init_event(InitRenderEvent::Step3Header)?;
            }
            InitEvent::Step3ScanCompleted { success, error } => {
                scan_needed = true;
                let step3_result = Step3Result::Scanned { success, error };
                view.render_init_event(InitRenderEvent::Step3Result(step3_result))?;
            }
            InitEvent::Step3Skipped { reason } => {
                let cli_reason = match reason {
                    agtrace_runtime::SkipReason::RecentlyScanned { elapsed } => {
                        SkipReason::RecentlyScanned { elapsed }
                    }
                };
                let step3_result = Step3Result::Skipped { reason: cli_reason };
                view.render_init_event(InitRenderEvent::Step3Result(step3_result))?;
            }
            InitEvent::Step4Header => {
                view.render_init_event(InitRenderEvent::Step4Header)?;
            }
            InitEvent::Step4NoSessions { all_projects } => {
                view.render_init_event(InitRenderEvent::Step4NoSessions { all_projects })?;
            }
            InitEvent::Step4SessionsFound {
                sessions: _,
                all_projects,
            } => {
                let db_path = ctx.data_dir().join("agtrace.db");
                let db = Database::open(&db_path)?;
                let effective_hash = if all_projects {
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
                    all_projects,
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
            InitEvent::NextSteps { session_id } => {
                view.render_init_event(InitRenderEvent::NextSteps { session_id })?;
            }
        }
        Ok(())
    })?;

    if scan_needed {
        super::index::handle(ctx, "all".to_string(), false, true, view)?;
    }

    Ok(())
}
