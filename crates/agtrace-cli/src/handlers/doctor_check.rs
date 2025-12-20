use crate::presentation::presenters;
use crate::presentation::renderers::TraceView;
use crate::presentation::view_models::{DoctorCheckResultViewModel, DoctorCheckStatus};
use agtrace_providers::{create_provider, detect_provider_from_path};
use agtrace_runtime::{CheckStatus, DoctorService};
use anyhow::Result;

pub fn handle(
    file_path: String,
    provider_override: Option<String>,
    view: &dyn TraceView,
) -> Result<()> {
    let (provider, provider_name) = if let Some(name) = provider_override {
        let provider = create_provider(&name)?;
        (provider, name)
    } else {
        let provider = detect_provider_from_path(&file_path)?;
        let name = format!("{} (auto-detected)", provider.name());
        (provider, name)
    };

    let result = DoctorService::check_file(&file_path, provider.as_ref(), &provider_name)?;

    let result_vm = DoctorCheckResultViewModel {
        file_path: result.file_path,
        provider_name: result.provider_name,
        status: match result.status {
            CheckStatus::Success => DoctorCheckStatus::Success,
            CheckStatus::Failure => DoctorCheckStatus::Failure,
        },
        events: presenters::present_events(&result.events),
        error_message: result.error_message,
    };

    view.render_doctor_check(&result_vm)?;

    if matches!(result_vm.status, DoctorCheckStatus::Failure) {
        anyhow::bail!("Validation failed");
    }

    Ok(())
}
