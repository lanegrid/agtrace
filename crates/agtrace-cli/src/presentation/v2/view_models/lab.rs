use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LabExportViewModel {
    pub exported_count: usize,
    pub output_path: String,
}
