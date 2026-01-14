use serde::Serialize;
use std::fmt;

use super::{CreateView, ViewMode};

// --------------------------------------------------------
// Data Definitions (ViewModels)
// --------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct WorktreeListViewModel {
    pub repository_hash: String,
    pub worktrees: Vec<WorktreeEntryViewModel>,
}

#[derive(Debug, Serialize)]
pub struct WorktreeEntryViewModel {
    pub name: String,
    pub path: String,
    pub session_count: usize,
    pub last_active: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WorktreeSessionsViewModel {
    pub repository_hash: String,
    pub total_sessions: usize,
    pub groups: Vec<WorktreeGroupViewModel>,
}

#[derive(Debug, Serialize)]
pub struct WorktreeGroupViewModel {
    pub name: String,
    pub path: String,
    pub sessions: Vec<WorktreeSessionViewModel>,
}

#[derive(Debug, Serialize)]
pub struct WorktreeSessionViewModel {
    pub id: String,
    pub id_short: String,
    pub time: String,
    pub snippet: String,
}

// --------------------------------------------------------
// CreateView Trait Implementations (Bridge to Views)
// --------------------------------------------------------

impl CreateView for WorktreeListViewModel {
    fn create_view<'a>(&'a self, mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::views::worktree::WorktreeListView;
        Box::new(WorktreeListView::new(self, mode))
    }
}

impl CreateView for WorktreeSessionsViewModel {
    fn create_view<'a>(&'a self, mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::views::worktree::WorktreeSessionsView;
        Box::new(WorktreeSessionsView::new(self, mode))
    }
}

// --------------------------------------------------------
// Display Trait (for backward compatibility and default rendering)
// --------------------------------------------------------

impl fmt::Display for WorktreeListViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}

impl fmt::Display for WorktreeSessionsViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}
