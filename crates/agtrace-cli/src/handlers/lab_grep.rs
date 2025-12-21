use crate::presentation::presenters;
use crate::presentation::renderers::TraceView;
use agtrace_runtime::{AgTrace, SessionFilter};
use anyhow::Result;

pub fn handle(
    workspace: &AgTrace,
    pattern: String,
    limit: Option<usize>,
    source: Option<String>,
    json_output: bool,
    view: &dyn TraceView,
) -> Result<()> {
    let mut filter = SessionFilter::new().limit(1000);
    if let Some(src) = source {
        filter = filter.source(src);
    }

    let sessions = workspace.sessions().list(filter)?;
    let mut matches = Vec::new();
    let max_matches = limit.unwrap_or(50);

    'outer: for session_summary in sessions {
        let session = workspace.sessions().find(&session_summary.id)?;
        let events = session.events()?;

        for event in events {
            let payload_str = serde_json::to_string(&event.payload)?;

            if payload_str.contains(&pattern) {
                let vm = presenters::present_event(&event);
                matches.push(vm);

                if matches.len() >= max_matches {
                    break 'outer;
                }
            }
        }
    }

    view.render_lab_grep(&matches, &pattern, json_output)?;

    Ok(())
}
