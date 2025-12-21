use agtrace_runtime::{AgTrace, DiscoveryEvent, StreamEvent, WorkspaceEvent};
use anyhow::Result;
use chrono::Local;
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "agtrace-debug")]
#[command(about = "Debug tool for agtrace event streams", long_about = None)]
struct Cli {
    #[arg(long, default_value = "~/.agtrace")]
    data_dir: String,

    #[arg(long)]
    provider: String,

    #[arg(long)]
    project_root: Option<String>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let data_dir = expand_tilde(&cli.data_dir);

    eprintln!("[DEBUG] Opening workspace at: {}", data_dir.display());
    let workspace = AgTrace::open(data_dir)?;

    let watch_service = workspace.watch_service();
    eprintln!("[DEBUG] Watching provider: {}", cli.provider);

    let mut builder = watch_service.watch_provider(&cli.provider)?;
    if let Some(root) = cli.project_root {
        let root_path = PathBuf::from(root);
        eprintln!("[DEBUG] Project root: {}", root_path.display());
        builder = builder.with_project_root(root_path);
    }

    let monitor = builder.start_background_scan()?;
    eprintln!("[DEBUG] Monitor started, listening for events... (Ctrl+C to exit)\n");

    loop {
        match monitor.receiver().recv_timeout(Duration::from_millis(500)) {
            Ok(event) => {
                let timestamp = Local::now().format("%H:%M:%S%.3f");
                match event {
                    WorkspaceEvent::Discovery(discovery) => match discovery {
                        DiscoveryEvent::NewSession { summary } => {
                            println!(
                                "[{}] DISCOVERY::NewSession {{ id: {}, provider: {}, snippet: {:?} }}",
                                timestamp, summary.id, summary.provider, summary.snippet
                            );
                        }
                        DiscoveryEvent::SessionUpdated {
                            session_id,
                            provider_name,
                            is_new,
                        } => {
                            println!(
                                "[{}] DISCOVERY::SessionUpdated {{ id: {}, provider: {}, is_new: {} }}",
                                timestamp, session_id, provider_name, is_new
                            );
                        }
                        DiscoveryEvent::SessionRemoved { session_id } => {
                            println!(
                                "[{}] DISCOVERY::SessionRemoved {{ id: {} }}",
                                timestamp, session_id
                            );
                        }
                    },
                    WorkspaceEvent::Stream(stream) => match stream {
                        StreamEvent::Attached { session_id, path } => {
                            println!(
                                "[{}] STREAM::Attached {{ id: {}, path: {} }}",
                                timestamp,
                                session_id,
                                path.display()
                            );
                        }
                        StreamEvent::Events { events } => {
                            println!(
                                "[{}] STREAM::Events {{ count: {} }}",
                                timestamp,
                                events.len()
                            );
                            for event in events {
                                println!(
                                    "  - {} | {:?}",
                                    event.timestamp.format("%H:%M:%S"),
                                    event.payload
                                );
                            }
                        }
                        StreamEvent::Disconnected { reason } => {
                            println!(
                                "[{}] STREAM::Disconnected {{ reason: {} }}",
                                timestamp, reason
                            );
                        }
                    },
                    WorkspaceEvent::Error(msg) => {
                        println!("[{}] ERROR: {}", timestamp, msg);
                    }
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Continue checking for events
                continue;
            }
            Err(e) => {
                eprintln!("[DEBUG] Receiver error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }
    PathBuf::from(path)
}
