use crate::cli::parser::ListArgs;
use crate::utils::{ParaError, Result};

pub fn execute(args: ListArgs) -> Result<()> {
    println!("List command would execute with args: {:?}", args);

    Err(ParaError::not_implemented("list command"))
}
