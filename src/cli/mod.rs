pub mod commands;
pub mod completion;
pub mod parser;

#[cfg(test)]
mod tests;

pub use parser::{Cli, Commands};

use crate::config::ConfigManager;
use crate::utils::{ParaError, Result};

pub fn execute_command(cli: Cli) -> Result<()> {
    execute_command_with_config(cli, None)
}

pub fn execute_command_with_config(
    cli: Cli,
    test_config: Option<crate::config::Config>,
) -> Result<()> {
    // Load config once for all commands that need it
    let config =
        match cli.command {
            Some(Commands::Config(_))
            | Some(Commands::Completion(_))
            | Some(Commands::Init)
            | Some(Commands::CompletionSessions)
            | Some(Commands::CompletionBranches)
            | Some(Commands::Monitor(_))
            | None => None,
            _ => match test_config {
                Some(cfg) => Some(cfg),
                None => Some(ConfigManager::load_or_create().map_err(|e| {
                    ParaError::config_error(format!("Failed to load config: {}", e))
                })?),
            },
        };

    match cli.command {
        Some(Commands::Start(args)) => {
            args.validate()?;
            commands::start::execute(config.unwrap(), args)
        }
        Some(Commands::Dispatch(args)) => {
            args.validate()?;
            commands::dispatch::execute(config.unwrap(), args)
        }
        Some(Commands::Finish(args)) => {
            args.validate()?;
            commands::finish::execute(config.unwrap(), args)
        }
        Some(Commands::Cancel(args)) => commands::cancel::execute(config.unwrap(), args),
        Some(Commands::Clean(args)) => commands::clean::execute(config.unwrap(), args),
        Some(Commands::List(args)) => commands::list::execute(config.unwrap(), args),
        Some(Commands::Resume(args)) => commands::resume::execute(config.unwrap(), args),
        Some(Commands::Recover(args)) => commands::recover::execute(config.unwrap(), args),
        Some(Commands::Config(args)) => commands::config::execute(args),
        Some(Commands::Completion(args)) => commands::completion::execute(args),
        Some(Commands::Init) => commands::init::execute(),
        Some(Commands::Mcp(args)) => commands::mcp::handle_mcp_command(args),
        Some(Commands::CompletionSessions) => commands::completion_sessions::execute(),
        Some(Commands::CompletionBranches) => commands::completion_branches::execute(),
        Some(Commands::Monitor(args)) => commands::monitor::execute(args),
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
    println!("  cancel      Cancel current session");
    println!("  clean       Remove all active sessions");
    println!("  list, ls    List active sessions");
    println!("  resume      Resume a session in IDE");
    println!("  recover     Recover cancelled session");
    println!("  config      Setup configuration");
    println!("  mcp         Setup Model Context Protocol integration");
    println!("  monitor     Monitor active sessions with live updates");
    println!("  completion  Generate shell completion script");
    println!("  init        Initialize shell completions automatically");
    println!("  help        Print this message or the help of the given subcommand(s)");
    println!();
    println!("Use 'para <command> --help' for more information on a specific command.");
}
