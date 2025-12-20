use crate::presentation::presenters;
use crate::presentation::renderers::TraceView;
use crate::presentation::view_models::{DoctorCheckResultViewModel, DoctorCheckStatus};
use agtrace_providers::{create_provider, detect_provider_from_path, ImportContext, LogProvider};
use anyhow::Result;
use std::path::Path;

pub fn handle(
    file_path: String,
    provider_override: Option<String>,
    view: &dyn TraceView,
) -> Result<()> {
    let path = Path::new(&file_path);

    if !path.exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    // Auto-detect or use specified provider
    let (provider, provider_name): (Box<dyn LogProvider>, String) =
        if let Some(name) = provider_override {
            let provider = create_provider(&name)?;
            (provider, name)
        } else {
            let provider = detect_provider_from_path(&file_path)?;
            let name = format!("{} (auto-detected)", provider.name());
            (provider, name)
        };

    let context = ImportContext {
        project_root_override: None,
        session_id_prefix: None,
        all_projects: true,
    };

    let result_vm = match provider.normalize_file(path, &context) {
        Ok(events) => {
            let event_vms = presenters::present_events(&events);
            DoctorCheckResultViewModel {
                file_path: file_path.clone(),
                provider_name: provider_name.clone(),
                status: DoctorCheckStatus::Success,
                events: event_vms,
                error_message: None,
            }
        }
        Err(e) => DoctorCheckResultViewModel {
            file_path: file_path.clone(),
            provider_name: provider_name.clone(),
            status: DoctorCheckStatus::Failure,
            events: vec![],
            error_message: Some(format!("{:#}", e)),
        },
    };

    view.render_doctor_check(&result_vm)?;

    if matches!(result_vm.status, DoctorCheckStatus::Failure) {
        anyhow::bail!("Validation failed");
    }

    Ok(())
}
