mod cli;
mod config;
mod utils;

use clap::Parser;
use cli::{Cli, execute_command};

fn main() {
    let cli = Cli::parse();
    
    if let Err(e) = execute_command(cli) {
        eprintln!("para: {}", e);
        std::process::exit(1);
    }
}