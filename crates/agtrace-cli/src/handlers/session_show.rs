use crate::args::{OutputFormat, ViewModeArgs};
use crate::presentation::presenters;
use crate::presentation::renderers::ConsoleRenderer;
use crate::presentation::renderers::Renderer as _;
use agtrace_engine::assemble_session;
use agtrace_runtime::{AgTrace, SessionFilter, TokenLimits};
use anyhow::{Context, Result};

pub fn handle(
    workspace: &AgTrace,
    session_id: String,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    let session_ops = workspace.sessions();
    let session_meta = session_ops.find(&session_id)?;

    // Load and normalize events
    let all_events = session_meta.events()?;

    // Assemble session from events
    let session = assemble_session(&all_events)
        .with_context(|| format!("Failed to assemble session: {}", session_id))?;

    // Use a default model name for now
    // TODO: Extract actual model from session metadata or provider-specific data
    let model_name_display = "Claude 3.5 Sonnet".to_string();
    let model_name_key = "claude-sonnet-4-5".to_string(); // For token limit lookup

    // Get provider from database by looking up session
    let filter = SessionFilter::default();
    let session_summaries = workspace.sessions().list(filter)?;
    let provider = session_summaries
        .iter()
        .find(|s| s.id == session_id)
        .map(|s| s.provider.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let token_limits = TokenLimits::new();
    let max_context = token_limits
        .get_limit(&model_name_key)
        .map(|spec| spec.effective_limit() as u32);

    // Present session analysis with provider and model info
    let result =
        presenters::present_session_analysis(&session, &provider, &model_name_display, max_context);

    // Render output
    let v2_format = crate::presentation::OutputFormat::from(format);
    let resolved_view_mode = view_mode.resolve();
    let renderer = ConsoleRenderer::new(v2_format, resolved_view_mode);
    renderer.render(result)?;

    Ok(())
}
