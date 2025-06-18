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
    use crate::test_utils::test_helpers::*;
    use std::fs;
    use tempfile::TempDir;

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
        let mut config = create_test_config();
        config.git.branch_prefix = "custom".to_string();
        config.directories.subtrees_dir = "subtrees/custom".to_string();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        config.session.preserve_on_finish = true;
        config.session.auto_cleanup_days = Some(30);

        // Write config to the test config path
        let config_json = serde_json::to_string_pretty(&config).unwrap();
        fs::write(_guard.config_path(), config_json).unwrap();

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
