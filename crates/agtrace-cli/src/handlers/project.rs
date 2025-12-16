use crate::context::ExecutionContext;
use agtrace_types::discover_project_root;
use anyhow::Result;

pub fn handle(ctx: &ExecutionContext, project_root: Option<String>) -> Result<()> {
    let db = ctx.db()?;
    let project_root_path = discover_project_root(project_root.as_deref())?;
    let project_hash = agtrace_types::project_hash_from_root(&project_root_path.to_string_lossy());

    println!("Project root: {}", project_root_path.display());
    println!("Project hash: {}", project_hash);
    println!();

    // List all projects from database
    println!("Registered projects:");
    println!(
        "{:<20} {:<50} {:<10} LAST SCANNED",
        "HASH (short)", "ROOT PATH", "SESSIONS"
    );
    println!("{}", "-".repeat(120));

    let projects = db.list_projects()?;
    for project in projects {
        let session_count = db.count_sessions_for_project(&project.hash)?;
        let hash_short = if project.hash.len() > 16 {
            format!("{}...", &project.hash[..16])
        } else {
            project.hash.clone()
        };

        println!(
            "{:<20} {:<50} {:<10} {}",
            hash_short,
            project.root_path.unwrap_or_else(|| "(unknown)".to_string()),
            session_count,
            project
                .last_scanned_at
                .unwrap_or_else(|| "(never)".to_string())
        );
    }

    Ok(())
}
