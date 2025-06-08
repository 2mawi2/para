use crate::cli::parser::CancelArgs;
use crate::utils::{ParaError, Result};

pub fn execute(args: CancelArgs) -> Result<()> {
    validate_cancel_args(&args)?;

    println!("Cancel command would execute with args: {:?}", args);

    Err(ParaError::not_implemented("cancel command"))
}

fn validate_cancel_args(args: &CancelArgs) -> Result<()> {
    if let Some(ref session) = args.session {
        if session.is_empty() {
            return Err(ParaError::invalid_args(
                "Session identifier cannot be empty",
            ));
        }
    }

    Ok(())
}
