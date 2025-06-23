//! Container patch watcher for applying changes from containers to host

use crate::core::git::{FinishRequest, GitOperations, GitService};
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize)]
struct ContainerCommitMetadata {
    message: String,
    branch: Option<String>,
    session_name: String,
    author: AuthorInfo,
    timestamp: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct AuthorInfo {
    name: String,
    email: String,
}

/// Watch for container patches and apply them to the host repository
pub struct ContainerPatchWatcher {
    workspace_path: PathBuf,
    patch_path: PathBuf,
    metadata_path: PathBuf,
}

impl ContainerPatchWatcher {
    /// Create a new patch watcher for a specific workspace
    pub fn new(workspace_path: &Path) -> Self {
        let patch_path = workspace_path.join(".para/container-changes.patch");
        let metadata_path = workspace_path.join(".para/container-commit.json");

        Self {
            workspace_path: workspace_path.to_path_buf(),
            patch_path,
            metadata_path,
        }
    }

    /// Check once for patches and apply if found
    pub fn check_and_apply_patches(
        &self,
        git_service: &GitService,
        session_manager: &mut SessionManager,
    ) -> Result<bool> {
        if !self.patch_path.exists() || !self.metadata_path.exists() {
            return Ok(false);
        }

        println!("📦 Container patch detected, applying...");

        // Read metadata
        let metadata_content = fs::read_to_string(&self.metadata_path)
            .map_err(|e| ParaError::fs_error(format!("Failed to read metadata: {}", e)))?;

        let metadata: ContainerCommitMetadata = serde_json::from_str(&metadata_content)?;

        // Apply patch
        self.apply_patch(&metadata)?;

        // Create commit with proper authorship
        self.create_commit(&metadata)?;

        // Finish the session using git service
        let final_branch = self.finish_session_with_git(git_service, &metadata)?;

        // Update session status
        if let Ok(_session) = session_manager.load_state(&metadata.session_name) {
            let _ = session_manager
                .update_session_status(&metadata.session_name, SessionStatus::Review);
        }

        // Clean up patch files
        self.cleanup_patch_files()?;

        println!("✅ Container changes applied successfully");
        println!("  Session: {}", metadata.session_name);
        println!("  Branch: {}", final_branch);
        println!("  Commit: {}", metadata.message);

        Ok(true)
    }

    /// Watch continuously for patches (blocking)
    #[allow(dead_code)]
    pub fn watch_continuously(
        &self,
        git_service: &GitService,
        session_manager: &mut SessionManager,
    ) -> Result<()> {
        loop {
            if self.check_and_apply_patches(git_service, session_manager)? {
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }
        Ok(())
    }

    fn apply_patch(&self, _metadata: &ContainerCommitMetadata) -> Result<()> {
        let output = Command::new("git")
            .current_dir(&self.workspace_path)
            .args(["apply", "--cached", &self.patch_path.to_string_lossy()])
            .output()
            .map_err(|e| ParaError::git_error(format!("Failed to apply patch: {}", e)))?;

        if !output.status.success() {
            return Err(ParaError::git_error(format!(
                "Failed to apply patch: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    fn create_commit(&self, metadata: &ContainerCommitMetadata) -> Result<()> {
        let output = Command::new("git")
            .current_dir(&self.workspace_path)
            .args(["commit", "-m", &metadata.message])
            .env("GIT_AUTHOR_NAME", &metadata.author.name)
            .env("GIT_AUTHOR_EMAIL", &metadata.author.email)
            .env("GIT_COMMITTER_NAME", &metadata.author.name)
            .env("GIT_COMMITTER_EMAIL", &metadata.author.email)
            .output()
            .map_err(|e| ParaError::git_error(format!("Failed to create commit: {}", e)))?;

        if !output.status.success() {
            return Err(ParaError::git_error(format!(
                "Failed to create commit: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    fn finish_session_with_git(
        &self,
        git_service: &GitService,
        metadata: &ContainerCommitMetadata,
    ) -> Result<String> {
        // Use git service to finish the session properly
        let finish_request = FinishRequest {
            feature_branch: "HEAD".to_string(), // We're already on the right branch
            commit_message: metadata.message.clone(),
            target_branch_name: metadata.branch.clone(),
        };

        match git_service.finish_session(finish_request)? {
            crate::core::git::FinishResult::Success { final_branch } => Ok(final_branch),
        }
    }

    fn cleanup_patch_files(&self) -> Result<()> {
        if self.patch_path.exists() {
            fs::remove_file(&self.patch_path)
                .map_err(|e| ParaError::fs_error(format!("Failed to remove patch file: {}", e)))?;
        }

        if self.metadata_path.exists() {
            fs::remove_file(&self.metadata_path).map_err(|e| {
                ParaError::fs_error(format!("Failed to remove metadata file: {}", e))
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_patch_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let watcher = ContainerPatchWatcher::new(temp_dir.path());

        assert_eq!(watcher.workspace_path, temp_dir.path());
        assert!(watcher
            .patch_path
            .to_string_lossy()
            .contains("container-changes.patch"));
        assert!(watcher
            .metadata_path
            .to_string_lossy()
            .contains("container-commit.json"));
    }

    #[test]
    fn test_check_patches_no_files() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let mut session_manager = SessionManager::new(&config);

        let watcher = ContainerPatchWatcher::new(&git_service.repository().root);
        let result = watcher
            .check_and_apply_patches(&git_service, &mut session_manager)
            .unwrap();

        assert!(!result); // No patches found
    }

    #[test]
    fn test_metadata_parsing() {
        let metadata_json = r#"{
            "message": "Test commit",
            "branch": "test-branch",
            "session_name": "test-session",
            "author": {
                "name": "Test User",
                "email": "test@example.com"
            },
            "timestamp": "2023-01-01T00:00:00Z"
        }"#;

        let metadata: ContainerCommitMetadata = serde_json::from_str(metadata_json).unwrap();
        assert_eq!(metadata.message, "Test commit");
        assert_eq!(metadata.session_name, "test-session");
        assert_eq!(metadata.author.name, "Test User");
    }
}
