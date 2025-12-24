use agtrace_cli::{Cli, run};
use clap::Parser;

fn main() {
    // Reset SIGPIPE to default behavior to prevent panic on broken pipe
    // (e.g., when piping to `head` or `less` that exits early)
    #[cfg(unix)]
    reset_sigpipe();

    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}
