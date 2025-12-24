use serde::Serialize;
use std::fmt;

use super::{CreateView, ViewMode};

// --------------------------------------------------------
// Data Definitions (ViewModels)
// --------------------------------------------------------

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

// --------------------------------------------------------
// CreateView Trait Implementations (Bridge to Views)
// --------------------------------------------------------

impl CreateView for ProjectListViewModel {
    fn create_view<'a>(&'a self, mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::views::project::ProjectListView;
        Box::new(ProjectListView::new(self, mode))
    }
}

// --------------------------------------------------------
// Display Trait (for backward compatibility and default rendering)
// --------------------------------------------------------

impl fmt::Display for ProjectListViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}
