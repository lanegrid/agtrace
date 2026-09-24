use crate::args::{OutputFormat, ViewModeArgs};
use crate::handlers::HandlerContext;
use crate::presentation::presenters;
use agtrace_sdk::Client;
use agtrace_sdk::types::{AgentId, ContextEvidence, ContextWindow};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

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

    // Get child sessions (subagents spawned from this session)
    let children = session_handle.child_sessions().unwrap_or_default();

    // Use assemble_all() to get all streams (Main + Sidechain + Subagent)
    let sessions = session_handle
        .assemble_all()
        .with_context(|| format!("Failed to assemble session: {}", session_id))?;

    let log_files: Vec<String> = session_handle
        .raw_files()?
        .into_iter()
        .map(|f| f.path)
        .collect();

    // Model and context window come from the events (per agent), resolved by the
    // context resolver against the workspace model catalog.
    let events = session_handle.events()?;
    let catalog = client.model_catalog();
    let mut evidence: BTreeMap<AgentId, ContextEvidence> = BTreeMap::new();
    for event in &events {
        evidence
            .entry(event.agent.clone())
            .or_default()
            .apply(event);
    }
    let windows: BTreeMap<AgentId, ContextWindow> = evidence
        .iter()
        .filter_map(|(agent, ev)| {
            let window =
                agtrace_sdk::utils::resolve_context_window(agent.provider(), ev, catalog.as_ref())?;
            Some((agent.clone(), window))
        })
        .collect();
    // Session-level model = model of the file-owner (non-subagent) agent.
    let model = evidence
        .iter()
        .find(|(agent, _)| !agent.is_claude_subagent())
        .and_then(|(_, ev)| ev.model.clone());

    // Present the whole session (all streams) as a single view model so that
    // every output format emits exactly one document.
    let view_model = presenters::present_session_detail(
        &sessions,
        &metadata.session_id,
        &metadata.provider,
        metadata.project_hash.as_ref(),
        metadata.project_root.as_deref(),
        metadata.spawned_by.as_ref(),
        model.as_deref(),
        &windows,
        log_files,
        &children,
    );

    ctx.render(view_model)
}
