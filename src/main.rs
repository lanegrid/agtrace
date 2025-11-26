use clap::Parser;

mod cli;
mod error;
mod model;
mod parser;
mod storage;

fn main() {
    let cli = cli::Cli::parse();

    if let Err(e) = cli::run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
