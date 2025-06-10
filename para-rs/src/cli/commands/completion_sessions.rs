use crate::config::Config;
use crate::core::session::SessionManager;
use crate::utils::Result;

pub fn execute() -> Result<()> {
    let config = Config::load_or_create()?;
    let session_manager = SessionManager::new(&config);
    
    match session_manager.list_sessions() {
        Ok(sessions) => {
            for session in sessions {
                println!("{}", session.name);
            }
        }
        Err(_) => {
            // Silent failure for completion compatibility
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_returns_ok() {
        let result = execute();
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_with_no_config() {
        let result = execute();
        assert!(result.is_ok());
    }

    #[test]
    fn test_silent_failure_behavior() {
        let result = execute();
        assert!(result.is_ok());
    }
}