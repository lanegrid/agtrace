use crate::args::{OutputFormat, ViewModeArgs};
use crate::handlers::HandlerContext;
use crate::presentation::presenters;
use agtrace_sdk::Client;
use agtrace_sdk::types::StreamId;
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

    // Use assemble_all() to get all streams (Main + Sidechain + Subagent)
    let mut sessions = session_handle
        .assemble_all()
        .with_context(|| format!("Failed to assemble session: {}", session_id))?;

    // Sort: Main first, then others by stream_id string
    sessions.sort_by(|a, b| match (&a.stream_id, &b.stream_id) {
        (StreamId::Main, StreamId::Main) => std::cmp::Ordering::Equal,
        (StreamId::Main, _) => std::cmp::Ordering::Less,
        (_, StreamId::Main) => std::cmp::Ordering::Greater,
        (a_id, b_id) => a_id.as_str().cmp(&b_id.as_str()),
    });

    let log_files: Vec<String> = session_handle
        .raw_files()?
        .into_iter()
        .map(|f| f.path)
        .collect();

    // TODO: Extract actual model from session metadata or provider-specific data
    let model_name_display = "Claude 3.5 Sonnet".to_string();
    let model_name_key = "claude-sonnet-4-5".to_string();

    let token_limits = agtrace_sdk::utils::default_token_limits();
    let max_context = token_limits
        .get_limit(&model_name_key)
        .map(|spec| spec.effective_limit() as u32);

    // Format spawn info from metadata (for Codex subagent sessions with separate files)
    let metadata_spawn_info = metadata
        .spawned_by
        .as_ref()
        .map(|ctx| {
            format!(
                " (spawned by Turn #{}, Step #{})",
                ctx.turn_index + 1,
                ctx.step_index + 1
            )
        })
        .unwrap_or_default();

    // Present each stream
    for (idx, session) in sessions.iter().enumerate() {
        if idx > 0 {
            // Add separator between streams with spawn context (for Claude Code sidechains)
            println!("\n{}", "─".repeat(80));
            let spawn_info = session
                .spawned_by
                .as_ref()
                .map(|ctx| {
                    format!(
                        " (spawned by Turn #{}, Step #{})",
                        ctx.turn_index + 1,
                        ctx.step_index + 1
                    )
                })
                .unwrap_or_default();
            println!(
                "Additional Stream: {}{}\n",
                session.stream_id.as_str(),
                spawn_info
            );
        }

        // Print spawn context for the main stream if it comes from DB metadata (Codex subagents)
        if idx == 0 && !metadata_spawn_info.is_empty() {
            println!("Spawned:{}", metadata_spawn_info);
            println!();
        }

        let view_model = presenters::present_session_analysis(
            session,
            &metadata.session_id,
            &metadata.provider,
            metadata.project_hash.as_ref(),
            metadata.project_root.as_deref(),
            &model_name_display,
            max_context,
            // Only show log_files for the first (main) stream
            if idx == 0 { log_files.clone() } else { vec![] },
        );

        ctx.render(view_model)?;
    }

    Ok(())
}
