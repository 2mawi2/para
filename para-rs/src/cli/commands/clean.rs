use crate::cli::parser::CleanArgs;
use crate::utils::{Result, ParaError};

pub fn execute(args: CleanArgs) -> Result<()> {
    println!("Clean command would execute with args: {:?}", args);
    
    Err(ParaError::not_implemented("clean command"))
}