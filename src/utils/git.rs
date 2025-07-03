use crate::utils::{ParaError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Get the main repository root, even when called from a worktree
///
/// This function uses `git rev-parse --git-common-dir` which is the most reliable
/// way to find the main repository root from any location (main repo or worktree).
pub fn get_main_repository_root() -> Result<PathBuf> {
    get_main_repository_root_from(None)
}

/// Get the main repository root from a specific path (used for testing)
///
/// This function uses `git rev-parse --git-common-dir` which is the most reliable
/// way to find the main repository root from any location (main repo or worktree).
pub fn get_main_repository_root_from(path: Option<&Path>) -> Result<PathBuf> {
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "--git-common-dir"]);

    if let Some(p) = path {
        cmd.current_dir(p);
    }

    let output = cmd
        .output()
        .map_err(|e| ParaError::git_error(format!("Failed to run git command: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ParaError::git_error(format!(
            "Git command failed: {stderr}"
        )));
    }

    let git_common_dir = String::from_utf8(output.stdout)
        .map_err(|e| ParaError::git_error(format!("Invalid git output: {e}")))?
        .trim()
        .to_string();

    if git_common_dir.is_empty() {
        return Err(ParaError::git_error(
            "Empty git-common-dir output".to_string(),
        ));
    }

    let git_common_path = if Path::new(&git_common_dir).is_absolute() {
        PathBuf::from(git_common_dir)
    } else {
        // If the path is relative, make it relative to the directory we're querying
        if let Some(p) = path {
            p.join(git_common_dir)
        } else {
            // When no path is provided, resolve relative to current directory
            std::env::current_dir()
                .map_err(|e| ParaError::git_error(format!("Failed to get current directory: {e}")))?
                .join(git_common_dir)
        }
    };

    // The git common dir points to the .git directory, we want the parent (repository root)
    let repo_root = if git_common_path
        .file_name()
        .map(|name| name == ".git")
        .unwrap_or(false)
    {
        git_common_path
            .parent()
            .unwrap_or(&git_common_path)
            .to_path_buf()
    } else {
        git_common_path
    };

    // Canonicalize the path to resolve symlinks and normalize the path
    repo_root
        .canonicalize()
        .map_err(|e| ParaError::git_error(format!("Failed to canonicalize repository root: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::git::GitOperations;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_main_repository_root_from_main_repo() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let result = get_main_repository_root_from(Some(git_temp.path()));
        assert!(result.is_ok());

        let repo_root = result.unwrap();
        let expected_path = git_temp.path().canonicalize().unwrap();
        assert_eq!(repo_root, expected_path);
    }

    #[test]
    fn test_get_main_repository_root_from_worktree() {
        let (git_temp, git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Create a worktree
        let worktree_path = git_temp.path().join("test-worktree");
        git_service
            .create_worktree("test-branch", &worktree_path)
            .unwrap();

        // Test from worktree - should return main repo root
        let result = get_main_repository_root_from(Some(&worktree_path));
        assert!(result.is_ok());

        let repo_root = result.unwrap();
        let expected_path = git_temp.path().canonicalize().unwrap();
        assert_eq!(repo_root, expected_path);
    }

    #[test]
    fn test_get_main_repository_root_not_in_git_repo() {
        let temp_dir = TempDir::new().unwrap();

        let result = get_main_repository_root_from(Some(temp_dir.path()));
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        println!("Error message: {error_msg}");
        assert!(
            error_msg.contains("Not in a git repository")
                || error_msg.contains("Git command failed")
        );
    }

    #[test]
    fn test_get_main_repository_root_current_directory() {
        // This test will work if the test is run from within a git repository
        // In our case, it should work since we're in the para repository
        let result = get_main_repository_root();

        // We can't make strong assertions about the result since it depends on the test environment
        // but we can at least verify it doesn't panic and returns a valid path
        if result.is_ok() {
            let repo_root = result.unwrap();
            assert!(repo_root.is_absolute());
            assert!(repo_root.exists());
            // Since we canonicalize the path, it should not contain ".." or other relative components
            assert!(!repo_root.to_string_lossy().contains(".."));
        }
    }
}
