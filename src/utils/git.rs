use crate::core::git::GitService;
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
        .map_err(|e| ParaError::git_error(format!("Failed to run git command: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ParaError::git_error(format!(
            "Git command failed: {}",
            stderr
        )));
    }

    let git_common_dir = String::from_utf8(output.stdout)
        .map_err(|e| ParaError::git_error(format!("Invalid git output: {}", e)))?
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
            PathBuf::from(git_common_dir)
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
        .map_err(|e| ParaError::git_error(format!("Failed to canonicalize repository root: {}", e)))
}

/// Utility for listing and processing archived branches with shared filtering logic
pub struct BranchLister<'a> {
    git_service: &'a GitService,
    branch_prefix: String,
}

impl<'a> BranchLister<'a> {
    pub fn new(git_service: &'a GitService, branch_prefix: &str) -> Self {
        Self {
            git_service,
            branch_prefix: branch_prefix.to_string(),
        }
    }

    /// List archived branches and apply a parser function to each branch
    /// Returns a sorted vector of parsed results (sorted by timestamp descending)
    pub fn list_archived_branches_with_parser<T, F>(&self, parser: F) -> Result<Vec<T>>
    where
        F: Fn(&str) -> Result<Option<T>>,
        T: HasTimestamp,
    {
        let archived_branches = self
            .git_service
            .branch_manager()
            .list_archived_branches(&self.branch_prefix)?;

        let mut entries = Vec::new();

        for archived_branch in archived_branches {
            if let Some(entry) = parser(&archived_branch)? {
                entries.push(entry);
            }
        }

        // Sort by timestamp in descending order (most recent first)
        entries.sort_by(|a, b| b.get_timestamp().cmp(a.get_timestamp()));
        Ok(entries)
    }
}

/// Trait for types that have a timestamp field for sorting
pub trait HasTimestamp {
    fn get_timestamp(&self) -> &str;
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
        println!("Error message: {}", error_msg);
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

    // Test types for BranchLister tests
    #[derive(Debug, Clone)]
    struct TestArchiveEntry {
        name: String,
        timestamp: String,
    }

    impl HasTimestamp for TestArchiveEntry {
        fn get_timestamp(&self) -> &str {
            &self.timestamp
        }
    }

    #[test]
    fn test_branch_lister_empty_list() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let branch_lister = BranchLister::new(&git_service, "test");

        let parser = |_branch: &str| -> Result<Option<TestArchiveEntry>> {
            // No branches to parse in empty repo
            Ok(None)
        };

        let entries = branch_lister.list_archived_branches_with_parser(parser).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_branch_lister_with_archived_branches() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let branch_manager = git_service.branch_manager();
        let initial_branch = git_service.repository().get_current_branch().unwrap();
        
        // Create some archived branches
        let archived_branches = vec![
            "test/archived/20240301-120000/session-1",
            "test/archived/20240302-130000/session-2", 
            "test/archived/20240303-140000/session-3",
        ];

        for branch in &archived_branches {
            branch_manager.create_branch(branch, &initial_branch).unwrap();
        }

        let branch_lister = BranchLister::new(&git_service, "test");

        let parser = |branch: &str| -> Result<Option<TestArchiveEntry>> {
            // Simple parser that extracts session name and timestamp
            if let Some(parts) = branch.split('/').collect::<Vec<_>>().get(2..) {
                if parts.len() >= 2 {
                    return Ok(Some(TestArchiveEntry {
                        name: parts[1].to_string(),
                        timestamp: parts[0].to_string(),
                    }));
                }
            }
            Ok(None)
        };

        let entries = branch_lister.list_archived_branches_with_parser(parser).unwrap();
        
        // Should have 3 entries
        assert_eq!(entries.len(), 3);
        
        // Should be sorted by timestamp descending (most recent first)
        assert_eq!(entries[0].timestamp, "20240303-140000");
        assert_eq!(entries[1].timestamp, "20240302-130000");
        assert_eq!(entries[2].timestamp, "20240301-120000");
        
        // Check session names
        assert_eq!(entries[0].name, "session-3");
        assert_eq!(entries[1].name, "session-2");
        assert_eq!(entries[2].name, "session-1");
    }

    #[test]
    fn test_branch_lister_with_parser_errors() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let branch_manager = git_service.branch_manager();
        let initial_branch = git_service.repository().get_current_branch().unwrap();
        
        // Create an archived branch
        branch_manager.create_branch("test/archived/20240301-120000/session-1", &initial_branch).unwrap();

        let branch_lister = BranchLister::new(&git_service, "test");

        let parser = |_branch: &str| -> Result<Option<TestArchiveEntry>> {
            // Parser that always fails
            Err(ParaError::git_error("Parser error".to_string()))
        };

        let result = branch_lister.list_archived_branches_with_parser(parser);
        assert!(result.is_err());
    }

    #[test]
    fn test_branch_lister_with_partial_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let branch_manager = git_service.branch_manager();
        let initial_branch = git_service.repository().get_current_branch().unwrap();
        
        // Create some archived branches
        let archived_branches = vec![
            "test/archived/20240301-120000/session-1",
            "test/archived/invalid-format/session-2", // This should be skipped
            "test/archived/20240303-140000/session-3",
        ];

        for branch in &archived_branches {
            branch_manager.create_branch(branch, &initial_branch).unwrap();
        }

        let branch_lister = BranchLister::new(&git_service, "test");

        let parser = |branch: &str| -> Result<Option<TestArchiveEntry>> {
            // Parser that only accepts valid timestamps
            if let Some(parts) = branch.split('/').collect::<Vec<_>>().get(2..) {
                if parts.len() >= 2 && parts[0].len() == 15 && parts[0].contains('-') {
                    return Ok(Some(TestArchiveEntry {
                        name: parts[1].to_string(),
                        timestamp: parts[0].to_string(),
                    }));
                }
            }
            Ok(None) // Skip invalid formats
        };

        let entries = branch_lister.list_archived_branches_with_parser(parser).unwrap();
        
        // Should have 2 entries (invalid format skipped)
        assert_eq!(entries.len(), 2);
        
        // Should be sorted by timestamp descending
        assert_eq!(entries[0].timestamp, "20240303-140000");
        assert_eq!(entries[1].timestamp, "20240301-120000");
    }
}
