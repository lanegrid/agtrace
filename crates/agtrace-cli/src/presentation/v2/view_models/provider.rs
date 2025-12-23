use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

use super::{CreateView, ViewMode};

// --------------------------------------------------------
// Data Definitions (ViewModels)
// --------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ProviderListViewModel {
    pub providers: Vec<ProviderEntry>,
}

#[derive(Debug, Serialize)]
pub struct ProviderEntry {
    pub name: String,
    pub enabled: bool,
    pub log_root: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct ProviderDetectedViewModel {
    pub providers: Vec<ProviderEntry>,
}

#[derive(Debug, Serialize)]
pub struct ProviderSetViewModel {
    pub provider: String,
    pub enabled: bool,
    pub log_root: PathBuf,
}

// --------------------------------------------------------
// CreateView Trait Implementations (Bridge to Views)
// --------------------------------------------------------

impl CreateView for ProviderListViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::v2::views::provider::ProviderListView;
        Box::new(ProviderListView::new(self))
    }
}

impl CreateView for ProviderDetectedViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::v2::views::provider::ProviderDetectedView;
        Box::new(ProviderDetectedView::new(self))
    }
}

impl CreateView for ProviderSetViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::v2::views::provider::ProviderSetView;
        Box::new(ProviderSetView::new(self))
    }
}

// --------------------------------------------------------
// Display Trait (for backward compatibility and default rendering)
// --------------------------------------------------------

impl fmt::Display for ProviderListViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}

impl fmt::Display for ProviderDetectedViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}

impl fmt::Display for ProviderSetViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}
