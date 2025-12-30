use crate::args::{OutputFormat, ViewModeArgs};
use agtrace_sdk::Client;
use agtrace_sdk::types::CheckStatus;
use anyhow::Result;
use std::path::Path;

pub fn handle(
    client: &Client,
    file_path: String,
    provider_override: Option<String>,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    use crate::presentation::presenters;
    use crate::presentation::{ConsoleRenderer, Renderer};

    let path = Path::new(&file_path);
    let result = client
        .system()
        .check_file(path, provider_override.as_deref())?;

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

    if matches!(result.status, CheckStatus::Failure) {
        anyhow::bail!("Validation failed");
    }

    Ok(())
}
