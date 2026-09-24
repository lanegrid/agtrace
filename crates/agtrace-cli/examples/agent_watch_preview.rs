//! Manual preview of the multi-agent watch TUI on the synthetic test workspace.
//!
//! ```sh
//! cargo run -p agtrace --example agent_watch_preview            # replay events live
//! cargo run -p agtrace --example agent_watch_preview -- --static # final state only
//! ```
//!
//! Events are folded one by one (every 250 ms) on a background thread, so the
//! generation-driven redraw path is exercised. Everything shown is synthetic.

#[path = "../tests/support/watch_fixture.rs"]
mod fixture;

use std::time::{Duration, Instant};

use agtrace::agent_watch::{SharedWorkspace, WorkspaceSource, run};
use agtrace::presentation::view_models::agent_watch::UiState;
use agtrace_sdk::workspace::WorkspaceView;
use chrono::{DateTime, FixedOffset, Utc};

/// Shared workspace whose clock starts at the fixture time and then runs in real time.
struct Preview {
    shared: SharedWorkspace,
    started: Instant,
}

impl WorkspaceSource for Preview {
    fn generation(&self) -> u64 {
        self.shared.generation()
    }

    fn with_view(&self, f: &mut dyn FnMut(&WorkspaceView)) {
        self.shared.with_view(f)
    }

    fn now(&self) -> DateTime<Utc> {
        fixture::now() + chrono::Duration::from_std(self.started.elapsed()).unwrap_or_default()
    }
}

fn main() -> anyhow::Result<()> {
    let replay = !std::env::args().any(|a| a == "--static");
    let shared = SharedWorkspace::new(if replay {
        WorkspaceView::new()
    } else {
        fixture::workspace()
    });
    if replay {
        let writer = shared.clone();
        std::thread::spawn(move || {
            for e in fixture::events() {
                std::thread::sleep(Duration::from_millis(250));
                writer.update(|v| v.apply(e, &fixture::resolve, fixture::now()));
            }
        });
    }
    let source = Preview {
        shared,
        started: Instant::now(),
    };
    let ui = UiState::new("synthetic preview", FixedOffset::east_opt(0).unwrap());
    run(&source, ui)
}
