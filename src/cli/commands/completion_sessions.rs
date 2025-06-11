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
    use std::path::PathBuf;
    use tempfile::TempDir;

    struct TestEnvironmentGuard {
        original_dir: PathBuf,
        original_state_dir: Option<String>,
        original_home: Option<String>,
        original_xdg_config_home: Option<String>,
    }

    impl TestEnvironmentGuard {
        fn new(
            git_temp: &TempDir,
            temp_dir: &TempDir,
        ) -> std::result::Result<Self, std::io::Error> {
            let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp"));
            let original_state_dir = std::env::var("PARA_STATE_DIR").ok();
            let original_home = std::env::var("HOME").ok();
            let original_xdg_config_home = std::env::var("XDG_CONFIG_HOME").ok();

            std::env::set_current_dir(git_temp.path())?;

            std::env::set_var("PARA_STATE_DIR", temp_dir.path());

            // Isolate config by setting HOME to temp directory
            std::env::set_var("HOME", temp_dir.path());
            std::env::remove_var("XDG_CONFIG_HOME");

            Ok(TestEnvironmentGuard {
                original_dir,
                original_state_dir,
                original_home,
                original_xdg_config_home,
            })
        }
    }

    impl Drop for TestEnvironmentGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original_dir);

            if let Some(ref state_dir) = self.original_state_dir {
                std::env::set_var("PARA_STATE_DIR", state_dir);
            } else {
                std::env::remove_var("PARA_STATE_DIR");
            }

            // Restore HOME
            match &self.original_home {
                Some(home) => std::env::set_var("HOME", home),
                None => std::env::remove_var("HOME"),
            }

            // Restore XDG_CONFIG_HOME
            match &self.original_xdg_config_home {
                Some(xdg) => std::env::set_var("XDG_CONFIG_HOME", xdg),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }
    }

    #[test]
    fn test_execute_returns_ok() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let result = execute();
        if let Err(e) = &result {
            eprintln!("Error in execute(): {}", e);
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
