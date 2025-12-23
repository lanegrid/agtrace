use serde::Serialize;
use std::fmt;

use super::{CreateView, ViewMode};

// --------------------------------------------------------
// Data Definitions (ViewModels)
// --------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct IndexResultViewModel {
    pub total_sessions: usize,
    pub scanned_files: usize,
    pub skipped_files: usize,
    pub mode: IndexMode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexMode {
    Update,
    Rebuild,
}

#[derive(Debug, Serialize)]
pub struct VacuumResultViewModel {
    pub success: bool,
}

// --------------------------------------------------------
// CreateView Trait Implementations (Bridge to Views)
// --------------------------------------------------------

impl CreateView for IndexResultViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::v2::views::index::IndexResultView;
        Box::new(IndexResultView::new(self))
    }
}

impl CreateView for VacuumResultViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::v2::views::index::VacuumResultView;
        Box::new(VacuumResultView::new(self))
    }
}

// --------------------------------------------------------
// Display Trait (for backward compatibility and default rendering)
// --------------------------------------------------------

impl fmt::Display for IndexResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}

impl fmt::Display for VacuumResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}
