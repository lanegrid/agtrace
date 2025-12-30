use crate::args::{OutputFormat, ViewModeArgs};
use crate::handlers::HandlerContext;
use crate::presentation::presenters;
use agtrace_sdk::Client;
use agtrace_sdk::types::{SessionFilter, TokenLimits, assemble_session};
use anyhow::{Context, Result};

pub fn handle(
    client: &Client,
    session_id: String,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    let ctx = HandlerContext::new(format, view_mode);

    let session_handle = client.sessions().get(&session_id)?;
    let all_events = session_handle.events()?;

    let session = assemble_session(&all_events)
        .with_context(|| format!("Failed to assemble session: {}", session_id))?;

    // TODO: Extract actual model from session metadata or provider-specific data
    let model_name_display = "Claude 3.5 Sonnet".to_string();
    let model_name_key = "claude-sonnet-4-5".to_string();

    let filter = SessionFilter::default();
    let session_summaries = client.sessions().list(filter)?;
    let provider = session_summaries
        .iter()
        .find(|s| s.id == session_id)
        .map(|s| s.provider.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let token_limits = TokenLimits::new();
    let max_context = token_limits
        .get_limit(&model_name_key)
        .map(|spec| spec.effective_limit() as u32);

    let view_model =
        presenters::present_session_analysis(&session, &provider, &model_name_display, max_context);

    ctx.render(view_model)
}
