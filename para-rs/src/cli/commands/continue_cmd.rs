use crate::utils::{Result, ParaError};

pub fn execute() -> Result<()> {
    println!("Continue command would execute");
    
    Err(ParaError::not_implemented("continue command"))
}