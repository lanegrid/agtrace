use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct ProjectListViewModel {
    pub current_root: String,
    pub current_hash: String,
    pub projects: Vec<ProjectEntryViewModel>,
}

#[derive(Debug, Serialize)]
pub struct ProjectEntryViewModel {
    pub hash: String,
    pub hash_short: String,
    pub root_path: Option<String>,
    pub session_count: usize,
    pub last_scanned: Option<String>,
}

impl fmt::Display for ProjectListViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Project root: {}", self.current_root)?;
        writeln!(f, "Project hash: {}", self.current_hash)?;
        writeln!(f)?;

        if self.projects.is_empty() {
            writeln!(f, "No projects registered.")?;
            return Ok(());
        }

        writeln!(f, "Registered projects:")?;
        writeln!(
            f,
            "{:<20} {:<50} {:<10} LAST SCANNED",
            "HASH (short)", "ROOT PATH", "SESSIONS"
        )?;
        writeln!(f, "{}", "-".repeat(120))?;

        for project in &self.projects {
            writeln!(
                f,
                "{:<20} {:<50} {:<10} {}",
                project.hash_short,
                project.root_path.as_deref().unwrap_or("(unknown)"),
                project.session_count,
                project.last_scanned.as_deref().unwrap_or("(never)")
            )?;
        }

        Ok(())
    }
}
