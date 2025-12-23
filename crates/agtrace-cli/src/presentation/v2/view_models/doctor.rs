use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

use super::{CreateView, ViewMode};

// --------------------------------------------------------
// Data Definitions (ViewModels)
// --------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct DiagnoseResultsViewModel {
    pub results: Vec<DiagnoseResultViewModel>,
}

#[derive(Debug, Serialize)]
pub struct DiagnoseResultViewModel {
    pub provider_name: String,
    pub total_files: usize,
    pub successful: usize,
    pub failures: HashMap<String, Vec<FailureExample>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct FailureExample {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct DoctorCheckResultViewModel {
    pub file_path: String,
    pub provider_name: String,
    pub status: CheckStatus,
    pub event_count: usize,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Success,
    Failure,
}

#[derive(Debug, Serialize)]
pub struct InspectResultViewModel {
    pub file_path: String,
    pub total_lines: usize,
    pub shown_lines: usize,
    pub lines: Vec<InspectLine>,
}

#[derive(Debug, Serialize)]
pub struct InspectLine {
    pub number: usize,
    pub content: String,
}

// --------------------------------------------------------
// CreateView Trait Implementations (Bridge to Views)
// --------------------------------------------------------

impl CreateView for DiagnoseResultsViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::v2::views::doctor::DiagnoseResultsView;
        Box::new(DiagnoseResultsView::new(self))
    }
}

impl CreateView for DiagnoseResultViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::v2::views::doctor::DiagnoseResultView;
        Box::new(DiagnoseResultView::new(self))
    }
}

impl CreateView for DoctorCheckResultViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::v2::views::doctor::DoctorCheckResultView;
        Box::new(DoctorCheckResultView::new(self))
    }
}

impl CreateView for InspectResultViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        use crate::presentation::v2::views::doctor::InspectResultView;
        Box::new(InspectResultView::new(self))
    }
}

// --------------------------------------------------------
// Display Trait (for backward compatibility and default rendering)
// --------------------------------------------------------

impl fmt::Display for DiagnoseResultsViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}

impl fmt::Display for DiagnoseResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}

impl fmt::Display for DoctorCheckResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}

impl fmt::Display for InspectResultViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.create_view(ViewMode::default()))
    }
}
