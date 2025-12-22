use crate::presentation::presenters;
use crate::presentation::renderers::TraceView;
use crate::presentation::view_models::{DoctorCheckResultViewModel, DoctorCheckStatus};
use agtrace_providers::{create_adapter, detect_adapter_from_path};
use agtrace_runtime::{AgTrace, CheckStatus};
use anyhow::Result;

pub fn handle(
    file_path: String,
    provider_override: Option<String>,
    view: &dyn TraceView,
) -> Result<()> {
    let (adapter, provider_name) = if let Some(name) = provider_override {
        let adapter = create_adapter(&name)?;
        (adapter, name)
    } else {
        let adapter = detect_adapter_from_path(&file_path)?;
        let name = format!("{} (auto-detected)", adapter.id());
        (adapter, name)
    };

    let result = AgTrace::check_file(&file_path, &adapter, &provider_name)?;

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
