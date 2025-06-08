pub mod parser;
pub mod commands;
pub mod completion;

#[cfg(test)]
mod tests;

pub use parser::{Cli, Commands};

use crate::utils::Result;

pub fn execute_command(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Start(args)) => {
            args.validate()?;
            commands::start::execute(args)
        },
        Some(Commands::Dispatch(args)) => {
            args.validate()?;
            commands::dispatch::execute(args)
        },
        Some(Commands::Finish(args)) => {
            args.validate()?;
            commands::finish::execute(args)
        },
        Some(Commands::Integrate(args)) => commands::integrate::execute(args),
        Some(Commands::Cancel(args)) => commands::cancel::execute(args),
        Some(Commands::Clean(args)) => commands::clean::execute(args),
        Some(Commands::List(args)) => commands::list::execute(args),
        Some(Commands::Resume(args)) => commands::resume::execute(args),
        Some(Commands::Recover(args)) => commands::recover::execute(args),
        Some(Commands::Continue) => commands::continue_cmd::execute(),
        Some(Commands::Config(args)) => commands::config::execute(args),
        Some(Commands::Completion(args)) => commands::completion::execute(args),
        None => {
            show_usage();
            Ok(())
        }
    }
}

fn show_usage() {
    println!("para - Parallel IDE Workflow Helper");
    println!();
    println!("A tool for managing multiple development sessions with git worktrees and IDEs.");
    println!();
    println!("Usage: para <COMMAND>");
    println!();
    println!("Commands:");
    println!("  start       Create a new development session");
    println!("  dispatch    Start Claude Code session with a prompt");
    println!("  finish      Complete current session with commit");
    println!("  integrate   Merge session into base branch");
    println!("  cancel      Cancel current session");
    println!("  clean       Remove all active sessions");
    println!("  list, ls    List active sessions");
    println!("  resume      Resume a session in IDE");
    println!("  recover     Recover cancelled session");
    println!("  continue    Complete merge after resolving conflicts");
    println!("  config      Setup configuration");
    println!("  completion  Generate shell completion script");
    println!("  help        Print this message or the help of the given subcommand(s)");
    println!();
    println!("Use 'para <command> --help' for more information on a specific command.");
}