use super::args::{Cli, Commands};
use super::handlers;
use crate::config::Config;
use crate::db::Database;
use crate::storage::Storage;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(cli: Cli) -> Result<()> {
    let data_dir = expand_tilde(&cli.data_dir);
    let storage = Storage::new(data_dir.clone());

    match cli.command {
        Commands::Scan {
            provider,
            force,
            verbose,
        } => {
            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;
            let config_path = data_dir.join("config.toml");
            let config = Config::load_from(&config_path)?;

            handlers::scan::handle(
                &db,
                &config,
                provider,
                cli.project_root,
                cli.all_projects,
                force,
                verbose,
            )
        }

        Commands::List {
            project_hash,
            source: _,
            limit,
            since: _,
            until: _,
        } => {
            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;

            // If --project-root is specified but no explicit hash, compute hash from project_root
            let effective_hash = if project_hash.is_none() && cli.project_root.is_some() {
                Some(crate::utils::project_hash_from_root(cli.project_root.as_ref().unwrap()))
            } else {
                project_hash
            };

            handlers::list::handle(
                &db,
                effective_hash,
                limit,
                cli.all_projects,
                &cli.format,
            )
        }

        Commands::View {
            session_id,
            raw,
            json,
            timeline,
            hide,
            only,
            full,
            short,
        } => {
            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;

            handlers::view::handle(&db, session_id, raw, json, timeline, hide, only, full, short)
        }

        Commands::Show {
            session_id,
            event_type: _,
            no_reasoning,
            no_tool,
            limit,
        } => handlers::show::handle(
            &storage,
            session_id,
            no_reasoning,
            no_tool,
            limit,
            &cli.format,
        ),

        Commands::Find {
            session_id,
            project_hash,
            event_id: _,
            text,
            event_type,
            limit,
        } => handlers::find::handle(
            &storage,
            session_id,
            project_hash,
            text,
            event_type,
            limit,
            cli.all_projects,
            &cli.format,
        ),

        Commands::Stats {
            project_hash,
            source,
            group_by: _,
            since: _,
            until: _,
        } => handlers::stats::handle(&storage, project_hash, source, cli.all_projects),

        Commands::Export {
            project_hash,
            session_id,
            source: _,
            event_type,
            since: _,
            until: _,
            out,
            format,
        } => handlers::export::handle(
            &storage,
            project_hash,
            session_id,
            event_type,
            cli.all_projects,
            out,
            format,
        ),

        Commands::Providers { command } => {
            let config_path = data_dir.join("config.toml");
            handlers::providers::handle(command, &config_path)
        }

        Commands::Project { project_root } => {
            let db_path = data_dir.join("agtrace.db");
            let db = Database::open(&db_path)?;
            handlers::project::handle(&db, project_root)
        }

        Commands::Diagnose {
            provider,
            verbose,
        } => {
            let config_path = data_dir.join("config.toml");
            let config = Config::load_from(&config_path)?;
            handlers::diagnose::handle(&config, provider, verbose)
        }

        Commands::Inspect {
            file_path,
            lines,
            format,
        } => handlers::inspect::handle(file_path, lines, format),

        Commands::Validate {
            file_path,
            provider,
        } => handlers::validate::handle(file_path, provider),

        Commands::Schema { provider, format } => handlers::schema::handle(provider, format),
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(&path[2..]);
        }
    }
    PathBuf::from(path)
}
