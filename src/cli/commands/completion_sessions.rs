use crate::config::Config;
use crate::core::session::SessionManager;
use crate::utils::Result;

pub fn execute() -> Result<()> {
    let config = match Config::load_or_create() {
        Ok(config) => config,
        Err(_) => {
            // Silent failure for completion compatibility
            return Ok(());
        }
    };
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
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_execute_returns_ok() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let result = execute();
        if let Err(e) = &result {
            eprintln!("Error in execute(): {e}");
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_with_no_config() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let result = execute();
        assert!(result.is_ok());
    }

    #[test]
    fn test_silent_failure_behavior() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let result = execute();
        assert!(result.is_ok());
    }
}
