use crate::presentation::v2::presenters;
use crate::presentation::v2::renderers::ConsoleRenderer;
use crate::presentation::v2::renderers::Renderer as _;
use agtrace_engine::assemble_session;
use agtrace_runtime::{AgTrace, SessionFilter, TokenLimits};
use anyhow::{Context, Result};

pub fn handle(workspace: &AgTrace, session_id: String, json: bool) -> Result<()> {
    let session_ops = workspace.sessions();
    let session_meta = session_ops.find(&session_id)?;

    // Load and normalize events
    let all_events = session_meta.events()?;

    // Assemble session from events
    let session = assemble_session(&all_events)
        .with_context(|| format!("Failed to assemble session: {}", session_id))?;

    // Use a default model name for now
    // TODO: Extract actual model from session metadata or provider-specific data
    let model_name = "Claude 3.5 Sonnet".to_string();

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
        .get_limit(&model_name)
        .map(|spec| spec.effective_limit() as u32);

    // Present session analysis with provider and model info
    let result =
        presenters::present_session_analysis(&session, &provider, &model_name, max_context);

    // Render output
    let renderer = ConsoleRenderer::new(json);
    renderer.render(result)?;

    Ok(())
}
