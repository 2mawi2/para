use crate::cli::parser::FinishArgs;
use crate::utils::{ParaError, Result};

pub fn execute(args: FinishArgs) -> Result<()> {
    validate_finish_args(&args)?;

    println!("Finish command would execute with args: {:?}", args);

    Err(ParaError::not_implemented("finish command"))
}

fn validate_finish_args(args: &FinishArgs) -> Result<()> {
    if args.message.trim().is_empty() {
        return Err(ParaError::invalid_args("Commit message cannot be empty"));
    }

    if let Some(ref branch) = args.branch {
        validate_branch_name(branch)?;
    }

    if let Some(ref session) = args.session {
        validate_session_identifier(session)?;
    }

    Ok(())
}

fn validate_branch_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(ParaError::invalid_args("Branch name cannot be empty"));
    }

    if name.starts_with('-') || name.ends_with('-') {
        return Err(ParaError::invalid_args(
            "Branch name cannot start or end with hyphen",
        ));
    }

    if name.contains("..") || name.contains("//") {
        return Err(ParaError::invalid_args(
            "Branch name contains invalid character sequence",
        ));
    }

    Ok(())
}

fn validate_session_identifier(session: &str) -> Result<()> {
    if session.is_empty() {
        return Err(ParaError::invalid_args(
            "Session identifier cannot be empty",
        ));
    }

    Ok(())
}
