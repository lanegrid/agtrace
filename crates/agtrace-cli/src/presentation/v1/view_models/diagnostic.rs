use std::collections::HashMap;

use crate::presentation::v1::view_models::EventViewModel;

#[derive(Debug, Clone)]
pub struct DiagnoseResultViewModel {
    pub provider_name: String,
    pub total_files: usize,
    pub successful: usize,
    pub failures: HashMap<String, Vec<FailureExampleViewModel>>,
}

#[derive(Debug, Clone)]
pub struct FailureExampleViewModel {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub enum DoctorCheckStatus {
    Success,
    Failure,
}

#[derive(Debug, Clone)]
pub struct DoctorCheckResultViewModel {
    pub file_path: String,
    pub provider_name: String,
    pub status: DoctorCheckStatus,
    pub events: Vec<EventViewModel>,
    pub error_message: Option<String>,
}
