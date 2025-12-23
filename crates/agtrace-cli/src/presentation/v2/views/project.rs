use std::fmt;

use crate::presentation::v2::view_models::ProjectListViewModel;

// --------------------------------------------------------
// Project List View
// --------------------------------------------------------

pub struct ProjectListView<'a> {
    data: &'a ProjectListViewModel,
}

impl<'a> ProjectListView<'a> {
    pub fn new(data: &'a ProjectListViewModel) -> Self {
        Self { data }
    }
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
