use crate::cli::parser::{ConfigArgs, ConfigCommands};
use crate::utils::{Result, ParaError};

pub fn execute(args: ConfigArgs) -> Result<()> {
    match args.command {
        Some(ConfigCommands::Setup) => execute_setup(),
        Some(ConfigCommands::Auto) => execute_auto(),
        Some(ConfigCommands::Show) => execute_show(),
        Some(ConfigCommands::Edit) => execute_edit(),
        None => execute_default(),
    }
}

fn execute_setup() -> Result<()> {
    println!("Config setup command would execute");
    Err(ParaError::not_implemented("config setup command"))
}

fn execute_auto() -> Result<()> {
    println!("Config auto command would execute");
    Err(ParaError::not_implemented("config auto command"))
}

fn execute_show() -> Result<()> {
    println!("Config show command would execute");
    Err(ParaError::not_implemented("config show command"))
}

fn execute_edit() -> Result<()> {
    println!("Config edit command would execute");
    Err(ParaError::not_implemented("config edit command"))
}

fn execute_default() -> Result<()> {
    println!("Config command (interactive) would execute");
    Err(ParaError::not_implemented("config command"))
}