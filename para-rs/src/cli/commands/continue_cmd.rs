use crate::utils::{ParaError, Result};

pub fn execute() -> Result<()> {
    println!("Continue command would execute");

    Err(ParaError::not_implemented("continue command"))
}
