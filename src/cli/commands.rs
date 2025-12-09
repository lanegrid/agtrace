use super::args::{Cli, Commands};
use super::handlers;
use crate::storage::Storage;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(cli: Cli) -> Result<()> {
    let data_dir = expand_tilde(&cli.data_dir);
    let storage = Storage::new(data_dir);

    match cli.command {
        Commands::Import {
            source,
            root,
            project_root,
            session_id_prefix,
            dry_run,
            out_jsonl,
        } => handlers::import::handle(
            &storage,
            source,
            root,
            project_root,
            session_id_prefix,
            cli.all_projects,
            dry_run,
            out_jsonl,
        ),

        Commands::List {
            project_hash,
            source,
            limit,
            since: _,
            until: _,
        } => handlers::list::handle(
            &storage,
            project_hash,
            source,
            limit,
            cli.all_projects,
            &cli.format,
        ),

        Commands::Show {
            session_id,
            event_type: _,
            no_reasoning,
            no_tool,
            limit,
        } => handlers::show::handle(&storage, session_id, no_reasoning, no_tool, limit, &cli.format),

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

        Commands::Providers { command } => handlers::providers::handle(command),

        Commands::Project { project_root } => handlers::project::handle(project_root),

        Commands::Status { project_root } => handlers::status::handle(project_root),
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
