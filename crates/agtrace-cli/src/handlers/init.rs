use crate::config::Config;
use crate::context::ExecutionContext;
use crate::presentation::formatters::init::{SkipReason, Step1Result, Step3Result};
use crate::presentation::renderers::models::InitRenderEvent;
use crate::presentation::renderers::TraceView;
use crate::types::OutputFormat;
use agtrace_index::Database;
use agtrace_types::project_hash_from_root;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

pub fn handle(ctx: &ExecutionContext, refresh: bool, view: &dyn TraceView) -> Result<()> {
    view.render_init_event(InitRenderEvent::Header)?;

    let data_dir = ctx.data_dir();
    let config_path = data_dir.join("config.toml");
    let db_path = data_dir.join("agtrace.db");

    let _config = if !config_path.exists() {
        view.render_init_event(InitRenderEvent::Step1Detecting)?;
        let detected = Config::detect_providers()?;

        if detected.providers.is_empty() {
            let step1_result = Step1Result::NoProvidersDetected;
            view.render_init_event(InitRenderEvent::Step1Result(step1_result))?;
            return Ok(());
        }

        let providers: HashMap<String, _> = detected
            .providers
            .iter()
            .map(|(name, cfg)| (name.clone(), cfg.log_root.clone()))
            .collect();

        detected.save_to(&config_path)?;
        let step1_result = Step1Result::DetectedProviders {
            providers,
            config_saved: true,
        };
        view.render_init_event(InitRenderEvent::Step1Result(step1_result))?;

        detected
    } else {
        view.render_init_event(InitRenderEvent::Step1Loading)?;
        let cfg = Config::load_from(&config_path)?;
        let step1_result = Step1Result::LoadedConfig {
            config_path: config_path.clone(),
        };
        view.render_init_event(InitRenderEvent::Step1Result(step1_result))?;
        cfg
    };

    view.render_init_event(InitRenderEvent::Step2Header)?;
    let db = Database::open(&db_path)?;
    view.render_init_event(InitRenderEvent::Step2Result {
        db_path: db_path.clone(),
    })?;

    let current_project_root = if let Some(root) = &ctx.project_root {
        root.display().to_string()
    } else {
        ".".to_string()
    };
    let current_project_hash = project_hash_from_root(&current_project_root);

    let should_scan = if refresh {
        true
    } else if let Ok(Some(project)) = db.get_project(&current_project_hash) {
        if let Some(last_scanned) = &project.last_scanned_at {
            if let Ok(last_time) = DateTime::parse_from_rfc3339(last_scanned) {
                let elapsed = Utc::now().signed_duration_since(last_time.with_timezone(&Utc));
                if elapsed < Duration::minutes(5) {
                    view.render_init_event(InitRenderEvent::Step3Header)?;
                    let step3_result = Step3Result::Skipped {
                        reason: SkipReason::RecentlyScanned { elapsed },
                    };
                    view.render_init_event(InitRenderEvent::Step3Result(step3_result))?;
                    false
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        }
    } else {
        true
    };

    if should_scan {
        view.render_init_event(InitRenderEvent::Step3Header)?;
        let scan_result = super::index::handle(ctx, "all".to_string(), false, true, view);

        match scan_result {
            Ok(_) => {}
            Err(e) => {
                let step3_result = Step3Result::Scanned {
                    success: false,
                    error: Some(format!("{}", e)),
                };
                view.render_init_event(InitRenderEvent::Step3Result(step3_result))?;
            }
        }
    }

    view.render_init_event(InitRenderEvent::Step4Header)?;

    let effective_hash = if ctx.all_projects {
        None
    } else {
        Some(current_project_hash.clone())
    };

    let sessions = db.list_sessions(effective_hash.as_deref(), 10)?;

    if sessions.is_empty() {
        view.render_init_event(InitRenderEvent::Step4NoSessions {
            all_projects: ctx.all_projects,
        })?;
        return Ok(());
    }

    super::session_list::handle(
        &db,
        effective_hash,
        10,
        ctx.all_projects,
        OutputFormat::Plain,
        None,
        None,
        None,
        true,
        ctx.data_dir(),
        ctx.project_root.as_ref().map(|p| p.display().to_string()),
        view,
    )?;

    if let Some(first_session) = sessions.first() {
        view.render_init_event(InitRenderEvent::NextSteps {
            session_id: first_session.id.clone(),
        })?;
    }

    Ok(())
}
