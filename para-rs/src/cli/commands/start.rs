use crate::cli::parser::StartArgs;
use crate::utils::{Result, ParaError};

pub fn execute(args: StartArgs) -> Result<()> {
    validate_start_args(&args)?;
    
    println!("Start command would execute with args: {:?}", args);
    
    Err(ParaError::not_implemented("start command"))
}

fn validate_start_args(args: &StartArgs) -> Result<()> {
    if let Some(ref name) = args.name {
        validate_session_name(name)?;
    }
    Ok(())
}

fn validate_session_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(ParaError::invalid_args("Session name cannot be empty"));
    }
    
    if name.len() > 50 {
        return Err(ParaError::invalid_args("Session name too long (max 50 characters)"));
    }
    
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(ParaError::invalid_args("Session name can only contain alphanumeric characters, hyphens, and underscores"));
    }
    
    Ok(())
}