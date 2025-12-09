use crate::cli::output::print_stats;
use crate::model::Source;
use crate::storage::Storage;
use crate::utils::discover_project_root;
use anyhow::Result;

fn parse_source(s: &str) -> Option<Source> {
    match s {
        "claude" => Some(Source::ClaudeCode),
        "codex" => Some(Source::Codex),
        "gemini" => Some(Source::Gemini),
        _ => None,
    }
}

pub fn handle(
    storage: &Storage,
    project_hash: Option<String>,
    source: Option<String>,
    all_projects: bool,
) -> Result<()> {
    let (effective_project_hash, all_projects) = if project_hash.is_some() {
        (project_hash.as_deref(), false)
    } else if all_projects {
        (None, true)
    } else {
        let project_root_path = discover_project_root(None)?;
        let current_project_hash =
            crate::utils::project_hash_from_root(&project_root_path.to_string_lossy());
        (Some(current_project_hash.leak() as &str), false)
    };

    let source_enum = source.as_deref().and_then(parse_source);
    let sessions =
        storage.list_sessions(effective_project_hash, source_enum, None, all_projects)?;

    print_stats(&sessions);

    Ok(())
}
