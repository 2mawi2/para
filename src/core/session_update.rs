// Temporary file to demonstrate usage of the new validation
use crate::utils::validation::validate_session_name;
use crate::utils::Result;

pub fn create_session_with_validation(name: &str) -> Result<()> {
    validate_session_name(name)?;
    // Rest of session creation logic would go here
    Ok(())
}