use serde::Serialize;
use std::fmt;

use super::{CreateView, ViewMode};

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

impl CreateView for ProjectListViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        // TODO: Implement different views for different modes in Phase 2
        Box::new(ProjectListView { data: self })
    }
}

struct ProjectListView<'a> {
    data: &'a ProjectListViewModel,
}

impl<'a> fmt::Display for ProjectListView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Project root: {}", self.data.current_root)?;
        writeln!(f, "Project hash: {}", self.data.current_hash)?;
        writeln!(f)?;

        if self.data.projects.is_empty() {
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

        for project in &self.data.projects {
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

impl fmt::Display for ProjectListViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", ProjectListView { data: self })
    }
}
