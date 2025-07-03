#[cfg(test)]
pub mod test_helpers {
    use crate::core::session::SessionState;
    use crate::utils::Result;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Parameters for creating a mock session state.
    /// This replaces the 6-parameter function with a more maintainable struct-based approach.
    #[derive(Debug)]
    pub struct SessionParams {
        pub session_id: String,
        pub branch: String,
        pub worktree_path: PathBuf,
        pub base_branch: String,
        pub merge_mode: String,
    }

    impl SessionParams {
        pub fn new(session_id: &str, branch: &str, worktree_path: &Path) -> Self {
            Self {
                session_id: session_id.to_string(),
                branch: branch.to_string(),
                worktree_path: worktree_path.to_path_buf(),
                base_branch: "main".to_string(),
                merge_mode: "squash".to_string(),
            }
        }

        pub fn with_base_branch(mut self, base_branch: &str) -> Self {
            self.base_branch = base_branch.to_string();
            self
        }

        pub fn with_merge_mode(mut self, merge_mode: &str) -> Self {
            self.merge_mode = merge_mode.to_string();
            self
        }
    }

    /// Create a mock session state with improved parameter handling.
    /// This replaces the old 6-parameter function with a cleaner struct-based approach.
    pub fn create_test_session_state(state_dir: &Path, params: SessionParams) -> Result<()> {
        fs::create_dir_all(state_dir)?;

        // Create a proper SessionState and serialize it to JSON
        let session_state = SessionState::new(
            params.session_id.clone(),
            params.branch,
            params.worktree_path,
        );

        let state_file = state_dir.join(format!("{}.state", params.session_id));
        let json_content = serde_json::to_string_pretty(&session_state).map_err(|e| {
            crate::utils::ParaError::invalid_config(format!(
                "Failed to serialize session state: {e}"
            ))
        })?;
        fs::write(state_file, json_content)?;

        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::test_utils::test_helpers::*;
        use std::fs;
        use tempfile::TempDir;

        #[test]
        fn test_session_params_new() {
            let temp_dir = TempDir::new().unwrap();
            let worktree_path = temp_dir.path().join("test-worktree");

            let params = SessionParams::new("test-session", "para/test-branch", &worktree_path);

            assert_eq!(params.session_id, "test-session");
            assert_eq!(params.branch, "para/test-branch");
            assert_eq!(params.worktree_path, worktree_path);
            assert_eq!(params.base_branch, "main"); // Default
            assert_eq!(params.merge_mode, "squash"); // Default
        }

        #[test]
        fn test_session_params_with_customization() {
            let temp_dir = TempDir::new().unwrap();
            let worktree_path = temp_dir.path().join("test-worktree");

            let params = SessionParams::new("test-session", "para/test-branch", &worktree_path)
                .with_base_branch("develop")
                .with_merge_mode("merge");

            assert_eq!(params.base_branch, "develop");
            assert_eq!(params.merge_mode, "merge");
        }

        #[test]
        fn test_create_test_session_state() -> Result<()> {
            let git_temp = TempDir::new().unwrap();
            let temp_dir = TempDir::new().unwrap();
            let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

            let config = create_test_config_with_dir(&temp_dir);
            let state_dir = std::path::PathBuf::from(&config.directories.state_dir);

            let worktree_path = temp_dir.path().join("test-worktree");
            fs::create_dir_all(&worktree_path)?;

            let params = SessionParams::new("test-session", "para/test-branch", &worktree_path);

            create_test_session_state(&state_dir, params)?;

            // Verify the state file was created
            let state_file = state_dir.join("test-session.state");
            assert!(state_file.exists());

            // Verify the content is valid JSON
            let content = fs::read_to_string(state_file)?;
            let session_state: crate::core::session::SessionState = serde_json::from_str(&content)?;
            assert_eq!(session_state.name, "test-session");
            assert_eq!(session_state.branch, "para/test-branch");

            Ok(())
        }
    }
}
