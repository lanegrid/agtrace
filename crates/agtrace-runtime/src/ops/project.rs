use crate::Result;
use agtrace_index::Database;

#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub hash: String,
    pub root_path: Option<String>,
    pub session_count: usize,
    pub last_scanned: Option<String>,
}

pub struct ProjectService<'a> {
    db: &'a Database,
}

impl<'a> ProjectService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectInfo>> {
        let projects = self.db.list_projects()?;
        let mut summaries = Vec::new();
        for project in projects {
            let session_count = self.db.count_sessions_for_project(project.hash.as_str())?;
            summaries.push(ProjectInfo {
                hash: project.hash.to_string(),
                root_path: project.root_path,
                session_count,
                last_scanned: project.last_scanned_at,
            });
        }
        Ok(summaries)
    }
}
