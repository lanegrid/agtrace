//! Live multi-agent workspace example
//!
//! This example demonstrates:
//! - Watching every agent of the current project (main sessions, teammates,
//!   subagents, Codex threads) with `Client::watch_workspace`
//! - Redrawing the agent tree whenever the view changes
//!
//! NOTE: This will run until Ctrl+C is pressed.
//! Start an agent session in this directory in another terminal to see it appear.

use agtrace_sdk::Client;
use agtrace_sdk::watch::WatchScope;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== agtrace SDK: Live Workspace Example ===\n");

    let client = Client::connect_default().await?;
    let project = std::env::current_dir()?;
    println!(
        "Watching agents of {} (Ctrl+C to exit)\n",
        project.display()
    );

    let mut live = client.watch_workspace(WatchScope::project(project))?;
    while live.changed().await.is_some() {
        let view = live.view();
        println!("--- {} agents ---", view.agents.len());
        for (id, depth) in view.tree() {
            let agent = &view.agents[id];
            let ctx = agent
                .window
                .as_ref()
                .map(|w| {
                    format!(
                        " {}% of {} [{}]",
                        agent.context.last_context_tokens * 100 / w.tokens.max(1),
                        w.tokens,
                        w.provenance()
                    )
                })
                .unwrap_or_default();
            println!(
                "{}{} {:?}{}",
                "  ".repeat(depth),
                agent.label(),
                agent.status,
                ctx
            );
        }
    }
    Ok(())
}
