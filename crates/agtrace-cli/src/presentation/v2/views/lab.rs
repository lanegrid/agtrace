use std::fmt;

use crate::presentation::v2::view_models::{CreateView, LabExportViewModel, ViewMode};

impl CreateView for LabExportViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(LabExportView { data: self })
    }
}

struct LabExportView<'a> {
    data: &'a LabExportViewModel,
}

impl<'a> fmt::Display for LabExportView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Exported {} events to {}",
            self.data.exported_count, self.data.output_path
        )
    }
}
