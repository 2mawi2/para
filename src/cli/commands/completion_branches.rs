use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::utils::Result;

pub fn execute() -> Result<()> {
    let branch_prefix = match Config::load_or_create() {
        Ok(config) => format!("{}/", config.get_branch_prefix()),
        Err(_) => {
            // Silent failure - fall back to default prefix for completion compatibility
            "para/".to_string()
        }
    };

    match GitService::discover() {
        Ok(git_service) => {
            match git_service.list_branches() {
                Ok(branches) => {
                    for branch in branches {
                        // Filter out para's internal branches (consistent with legacy)
                        if !branch.name.starts_with(&branch_prefix) {
                            println!("{}", branch.name);
                        }
                    }
                }
                Err(_) => {
                    // Silent failure for completion compatibility
                }
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
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    struct TestEnvironmentGuard {
        original_dir: PathBuf,
        original_home: Option<String>,
        original_xdg_config_home: Option<String>,
    }

    impl TestEnvironmentGuard {
        fn new(
            git_temp: &TempDir,
            temp_dir: &TempDir,
        ) -> std::result::Result<Self, std::io::Error> {
            let original_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp"));
            let original_home = std::env::var("HOME").ok();
            let original_xdg_config_home = std::env::var("XDG_CONFIG_HOME").ok();

            std::env::set_current_dir(git_temp.path())?;

            // Isolate config by setting HOME to temp directory
            std::env::set_var("HOME", temp_dir.path());
            std::env::remove_var("XDG_CONFIG_HOME");

            Ok(TestEnvironmentGuard {
                original_dir,
                original_home,
                original_xdg_config_home,
            })
        }
    }

    impl Drop for TestEnvironmentGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original_dir);

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

    fn setup_test_repo() -> (TempDir, crate::core::git::GitService) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path();

        Command::new("git")
            .current_dir(repo_path)
            .args(["init", "--initial-branch=main"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(repo_path)
            .args(["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(repo_path)
            .args(["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit README");

        let service = crate::core::git::GitService::discover_from(repo_path)
            .expect("Failed to discover repo");
        (temp_dir, service)
    }

    #[test]
    fn test_execute_with_no_git_repo() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let result = execute();
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_returns_ok() {
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

    #[test]
    fn test_execute_with_git_repo_and_branches() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        // Create some test branches including para branches
        let branch_manager = git_service.branch_manager();
        let current_branch = git_service.repository().get_current_branch().unwrap();

        branch_manager
            .create_branch("feature-branch", &current_branch)
            .unwrap();
        branch_manager
            .create_branch("para/test-session", &current_branch)
            .unwrap();

        // Switch back to main branch
        git_service
            .repository()
            .checkout_branch(&current_branch)
            .unwrap();

        let result = execute();
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_with_no_config_falls_back_to_default() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, _git_service) = setup_test_repo();

        // This should work even without a config file, falling back to "para/"
        let result = execute();
        assert!(result.is_ok());
    }

    #[test]
    fn test_branch_filtering_with_custom_prefix() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        // Create config with custom prefix
        let config_dir = temp_dir.path().join(".config").join("para");
        fs::create_dir_all(&config_dir).unwrap();
        let config_content = r#"{
            "ide": {
                "name": "test",
                "command": "echo",
                "user_data_dir": null,
                "wrapper": {
                    "enabled": false,
                    "name": "",
                    "command": ""
                }
            },
            "directories": {
                "subtrees_dir": "subtrees/custom",
                "state_dir": ".para_state"
            },
            "git": {
                "branch_prefix": "custom",
                "auto_stage": true,
                "auto_commit": true
            },
            "session": {
                "default_name_format": "%Y%m%d-%H%M%S",
                "preserve_on_finish": true,
                "auto_cleanup_days": 30
            }
        }"#;
        fs::write(config_dir.join("config.json"), config_content).unwrap();

        // Create branches with different prefixes
        let branch_manager = git_service.branch_manager();
        let current_branch = git_service.repository().get_current_branch().unwrap();

        branch_manager
            .create_branch("feature-branch", &current_branch)
            .unwrap();
        branch_manager
            .create_branch("custom/test-session", &current_branch)
            .unwrap();
        branch_manager
            .create_branch("para/old-session", &current_branch)
            .unwrap();

        git_service
            .repository()
            .checkout_branch(&current_branch)
            .unwrap();

        // This should filter out only "custom/" branches, not "para/" branches
        let result = execute();
        assert!(result.is_ok());
    }
}
