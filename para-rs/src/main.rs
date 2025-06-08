mod cli;
mod config;
mod core;
mod utils;

use clap::Parser;
use cli::{execute_command, Cli};

fn main() {
    let cli = Cli::parse();

    if let Err(e) = execute_command(cli) {
        eprintln!("para: {}", e);
        std::process::exit(1);
    }
}
