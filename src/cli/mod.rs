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
    let config =
        match cli.command {
            Some(Commands::Config(_))
            | Some(Commands::Completion(_))
            | Some(Commands::Init)
            | Some(Commands::CompletionSessions)
            | Some(Commands::CompletionBranches) => None,
            Some(Commands::Monitor(_)) | None => match test_config {
                Some(cfg) => Some(cfg),
                None => Some(ConfigManager::load_or_create().map_err(|e| {
                    ParaError::config_error(format!("Failed to load config: {}", e))
                })?),
            },
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
        Some(Commands::Monitor(args)) => commands::monitor::execute(config.unwrap(), args),
        Some(Commands::Status(args)) => commands::status::execute(config.unwrap(), args),
        None => commands::monitor::execute(config.unwrap(), crate::cli::parser::MonitorArgs {}),
    }
}
