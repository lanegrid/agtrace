use std::collections::HashMap;

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
