use serde::Serialize;

use crate::presentation::v2::renderers::ConsolePresentable;

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

impl ConsolePresentable for ProjectListViewModel {
    fn render_console(&self) {
        println!("Project root: {}", self.current_root);
        println!("Project hash: {}", self.current_hash);
        println!();

        if self.projects.is_empty() {
            println!("No projects registered.");
            return;
        }

        println!("Registered projects:");
        println!(
            "{:<20} {:<50} {:<10} LAST SCANNED",
            "HASH (short)", "ROOT PATH", "SESSIONS"
        );
        println!("{}", "-".repeat(120));

        for project in &self.projects {
            println!(
                "{:<20} {:<50} {:<10} {}",
                project.hash_short,
                project.root_path.as_deref().unwrap_or("(unknown)"),
                project.session_count,
                project.last_scanned.as_deref().unwrap_or("(never)")
            );
        }
    }
}
