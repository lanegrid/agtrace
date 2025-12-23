use std::fmt;

use crate::presentation::v2::view_models::{ProjectListViewModel, ViewMode};

// --------------------------------------------------------
// Project List View
// --------------------------------------------------------

pub struct ProjectListView<'a> {
    data: &'a ProjectListViewModel,
    mode: ViewMode,
}

impl<'a> ProjectListView<'a> {
    pub fn new(data: &'a ProjectListViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.data.current_hash)?;
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Current: {}", self.data.current_hash)?;
        writeln!(f, "{} projects registered", self.data.projects.len())?;
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

impl<'a> fmt::Display for ProjectListView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}
