use crate::args::{OutputFormat, ViewModeArgs};
use agtrace_providers::{create_adapter, detect_adapter_from_path};
use agtrace_runtime::AgTrace;
use anyhow::Result;

pub fn handle(
    file_path: String,
    provider_override: Option<String>,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    use crate::presentation::presenters;
    use crate::presentation::{ConsoleRenderer, Renderer};

    let (adapter, provider_name) = if let Some(name) = provider_override {
        let adapter = create_adapter(&name)?;
        (adapter, name)
    } else {
        let adapter = detect_adapter_from_path(&file_path)?;
        let name = format!("{} (auto-detected)", adapter.id());
        (adapter, name)
    };

    let result = AgTrace::check_file(&file_path, &adapter, &provider_name)?;

    let view_model = presenters::present_check_result(
        result.file_path,
        result.provider_name,
        result.status.clone(),
        result.events.len(),
        result.error_message,
    );

    let output_format = crate::presentation::OutputFormat::from(format);
    let renderer = ConsoleRenderer::new(output_format, view_mode.resolve());
    renderer.render(view_model)?;

    if matches!(result.status, agtrace_runtime::CheckStatus::Failure) {
        anyhow::bail!("Validation failed");
    }

    Ok(())
}
