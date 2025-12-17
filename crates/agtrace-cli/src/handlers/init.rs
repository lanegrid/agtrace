use crate::config::Config;
use crate::context::ExecutionContext;
use crate::display_model::init::{InitDisplay, SkipReason, Step1Result, Step3Result};
use crate::types::OutputFormat;
use crate::views::init;
use agtrace_index::Database;
use agtrace_types::project_hash_from_root;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

pub fn handle(ctx: &ExecutionContext, refresh: bool) -> Result<()> {
    init::print_init_header();

    let data_dir = ctx.data_dir();
    let config_path = data_dir.join("config.toml");
    let db_path = data_dir.join("agtrace.db");

    let mut display = InitDisplay::new(config_path.clone(), db_path.clone());

    let _config = if !config_path.exists() {
        init::print_step1_detecting();
        let detected = Config::detect_providers()?;

        if detected.providers.is_empty() {
            display = display.with_step1(Step1Result::NoProvidersDetected);
            init::print_step1_result(&display.step1);
            return Ok(());
        }

        let providers: HashMap<String, _> = detected
            .providers
            .iter()
            .map(|(name, cfg)| (name.clone(), cfg.log_root.clone()))
            .collect();

        detected.save_to(&config_path)?;
        display = display.with_step1(Step1Result::DetectedProviders {
            providers,
            config_saved: true,
        });
        init::print_step1_result(&display.step1);

        detected
    } else {
        init::print_step1_loading();
        let cfg = Config::load_from(&config_path)?;
        display = display.with_step1(Step1Result::LoadedConfig {
            config_path: config_path.clone(),
        });
        init::print_step1_result(&display.step1);
        cfg
    };

    init::print_step2_header();
    let db = Database::open(&db_path)?;
    init::print_step2_result(&display);

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
                    init::print_step3_header();
                    display = display.with_step3(Step3Result::Skipped {
                        reason: SkipReason::RecentlyScanned { elapsed },
                    });
                    init::print_step3_result(&display.step3);
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
        init::print_step3_header();
        let scan_result = super::index::handle(ctx, "all".to_string(), false, true);

        match scan_result {
            Ok(_) => {}
            Err(e) => {
                display = display.with_step3(Step3Result::Scanned {
                    success: false,
                    error: Some(format!("{}", e)),
                });
                init::print_step3_result(&display.step3);
            }
        }
    }

    init::print_step4_header();

    let effective_hash = if ctx.all_projects {
        None
    } else {
        Some(current_project_hash.clone())
    };

    let sessions = db.list_sessions(effective_hash.as_deref(), 10)?;

    if sessions.is_empty() {
        init::print_step4_no_sessions(ctx.all_projects);
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
    )?;

    if let Some(first_session) = sessions.first() {
        init::print_next_steps(&first_session.id);
    }

    Ok(())
}
