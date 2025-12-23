use std::path::Path;

use crate::presentation::v2::view_models::LabExportViewModel;

pub fn present_lab_export(exported_count: usize, output_path: &Path) -> LabExportViewModel {
    LabExportViewModel {
        exported_count,
        output_path: output_path.display().to_string(),
    }
}
