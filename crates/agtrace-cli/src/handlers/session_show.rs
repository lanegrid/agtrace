use crate::args::{OutputFormat, ViewModeArgs};
use crate::handlers::HandlerContext;
use crate::presentation::presenters;
use agtrace_sdk::Client;
use anyhow::{Context, Result};

pub fn handle(
    client: &Client,
    session_id: String,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    let ctx = HandlerContext::new(format, view_mode);

    let session_handle = client.sessions().get(&session_id)?;

    let metadata = session_handle
        .metadata()?
        .ok_or_else(|| anyhow::anyhow!("Session metadata not available"))?;

    let session = session_handle
        .assemble()
        .with_context(|| format!("Failed to assemble session: {}", session_id))?;

    // TODO: Extract actual model from session metadata or provider-specific data
    let model_name_display = "Claude 3.5 Sonnet".to_string();
    let model_name_key = "claude-sonnet-4-5".to_string();

    let token_limits = agtrace_sdk::utils::default_token_limits();
    let max_context = token_limits
        .get_limit(&model_name_key)
        .map(|spec| spec.effective_limit() as u32);

    let view_model = presenters::present_session_analysis(
        &session,
        &metadata.session_id,
        &metadata.provider,
        metadata.project_hash.as_ref(),
        &model_name_display,
        max_context,
    );

    ctx.render(view_model)
}
