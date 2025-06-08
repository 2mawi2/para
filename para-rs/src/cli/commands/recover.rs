use crate::cli::parser::RecoverArgs;
use crate::utils::{ParaError, Result};

pub fn execute(args: RecoverArgs) -> Result<()> {
    validate_recover_args(&args)?;

    println!("Recover command would execute with args: {:?}", args);

    Err(ParaError::not_implemented("recover command"))
}

fn validate_recover_args(args: &RecoverArgs) -> Result<()> {
    if let Some(ref session) = args.session {
        if session.is_empty() {
            return Err(ParaError::invalid_args(
                "Session identifier cannot be empty",
            ));
        }
    }

    Ok(())
}
