#[cfg(test)]
use crate::utils::Result;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[cfg(test)]
pub fn create_test_config() -> crate::config::Config {
    let mut config = crate::config::defaults::default_config();
    // Always use mock IDE commands in tests
    config.ide.name = "test-ide".to_string();
    config.ide.command = "echo".to_string();
    config.ide.wrapper.command = "echo".to_string();
    config
}

#[cfg(test)]
pub fn setup_test_repo() -> (TempDir, crate::core::git::GitService) {
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

#[cfg(test)]
pub struct TestEnvironmentGuard {
    original_dir: std::path::PathBuf,
}

#[cfg(test)]
impl TestEnvironmentGuard {
    pub fn new(
        git_temp: &TempDir,
        temp_dir: &TempDir,
    ) -> std::result::Result<Self, std::io::Error> {
        let original_dir = std::env::current_dir().unwrap_or_else(|_| {
            git_temp
                .path()
                .parent()
                .unwrap_or_else(|| std::path::Path::new("/tmp"))
                .to_path_buf()
        });

        std::env::set_current_dir(git_temp.path())?;

        // Create a mock config file for test isolation
        let test_config_path = temp_dir.path().join("para_config.json");
        let mock_config = create_test_config();
        let config_json = serde_json::to_string_pretty(&mock_config).unwrap();
        fs::write(&test_config_path, config_json).unwrap();

        Ok(TestEnvironmentGuard { original_dir })
    }
}

#[cfg(test)]
impl Drop for TestEnvironmentGuard {
    fn drop(&mut self) {
        if let Err(_e) = std::env::set_current_dir(&self.original_dir) {
            let _ = std::env::set_current_dir("/tmp");
        }
    }
}

#[derive(Debug)]
#[cfg(test)]
pub struct MockSessionStateParams {
    pub session_id: String,
    pub branch: String,
    pub worktree_path: String,
    pub base_branch: String,
    pub merge_mode: String,
}

#[cfg(test)]
impl MockSessionStateParams {
    pub fn new(
        session_id: &str,
        branch: &str,
        worktree_path: &str,
        base_branch: &str,
        merge_mode: &str,
    ) -> Self {
        Self {
            session_id: session_id.to_string(),
            branch: branch.to_string(),
            worktree_path: worktree_path.to_string(),
            base_branch: base_branch.to_string(),
            merge_mode: merge_mode.to_string(),
        }
    }
}

#[cfg(test)]
pub fn create_mock_session_state(
    state_dir: &Path,
    params: &MockSessionStateParams,
) -> Result<()> {
    use crate::core::session::SessionState;

    fs::create_dir_all(state_dir)?;

    // Create a proper SessionState and serialize it to JSON
    let session_state = SessionState::new(
        params.session_id.clone(),
        params.branch.clone(),
        std::path::PathBuf::from(&params.worktree_path),
    );

    let state_file = state_dir.join(format!("{}.state", params.session_id));
    let json_content = serde_json::to_string_pretty(&session_state).map_err(|e| {
        crate::utils::ParaError::invalid_config(format!(
            "Failed to serialize session state: {}",
            e
        ))
    })?;
    fs::write(state_file, json_content)?;

    Ok(())
}