use super::repository::{execute_git_command, execute_git_command_with_status, GitRepository};
use super::validation::GitValidator;
use crate::utils::error::{ParaError, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub commit: String,
    pub is_bare: bool,
}

pub struct WorktreeManager<'a> {
    repo: &'a GitRepository,
}

impl<'a> WorktreeManager<'a> {
    pub fn new(repo: &'a GitRepository) -> Self {
        Self { repo }
    }

    pub fn create_worktree(&self, branch_name: &str, path: &Path) -> Result<()> {
        self.validate_branch_name(branch_name)?;
        self.validate_worktree_path(path)?;

        if path.exists() {
            return Err(ParaError::git_operation(format!(
                "Worktree path already exists: {}",
                path.display()
            )));
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ParaError::git_operation(format!("Failed to create parent directory: {}", e))
            })?;
        }

        let path_str = path.to_string_lossy();

        let branch_exists = execute_git_command(
            self.repo,
            &[
                "rev-parse",
                "--verify",
                &format!("refs/heads/{}", branch_name),
            ],
        )
        .is_ok();

        if branch_exists {
            execute_git_command_with_status(
                self.repo,
                &["worktree", "add", &path_str, branch_name],
            )?;
        } else {
            execute_git_command_with_status(
                self.repo,
                &["worktree", "add", "-b", branch_name, &path_str, "HEAD"],
            )?;
        }

        self.validate_worktree(path)?;
        Ok(())
    }

    pub fn remove_worktree(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(ParaError::git_operation(format!(
                "Worktree path does not exist: {}",
                path.display()
            )));
        }

        let path_str = path.to_string_lossy();

        execute_git_command_with_status(self.repo, &["worktree", "remove", &path_str]).or_else(
            |_| {
                execute_git_command_with_status(
                    self.repo,
                    &["worktree", "remove", "--force", &path_str],
                )
            },
        )?;

        if path.exists() {
            std::fs::remove_dir_all(path).map_err(|e| {
                ParaError::git_operation(format!("Failed to remove worktree directory: {}", e))
            })?;
        }

        Ok(())
    }

    pub fn force_remove_worktree(&self, path: &Path) -> Result<()> {
        let path_str = path.to_string_lossy();

        let _ = execute_git_command_with_status(
            self.repo,
            &["worktree", "remove", "--force", &path_str],
        );

        if path.exists() {
            std::fs::remove_dir_all(path).map_err(|e| {
                ParaError::git_operation(format!(
                    "Failed to force remove worktree directory: {}",
                    e
                ))
            })?;
        }

        self.prune_worktrees()?;
        Ok(())
    }

    pub fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let output = execute_git_command(self.repo, &["worktree", "list", "--porcelain"])?;
        parse_worktree_output(&output)
    }
}

fn parse_worktree_output(output: &str) -> Result<Vec<WorktreeInfo>> {
    let mut worktrees = Vec::new();
    let lines: Vec<&str> = output.lines().map(|line| line.trim()).collect();

    let mut i = 0;
    while i < lines.len() {
        let block_start = i;

        // Find the end of the current worktree block (empty line or end of input)
        while i < lines.len() && !lines[i].is_empty() {
            i += 1;
        }

        if i > block_start {
            let block_lines = &lines[block_start..i];
            if let Ok(worktree) = parse_worktree_block(block_lines) {
                worktrees.push(worktree);
            }
        }

        // Skip empty lines
        while i < lines.len() && lines[i].is_empty() {
            i += 1;
        }
    }

    Ok(worktrees)
}

fn parse_worktree_block(lines: &[&str]) -> Result<WorktreeInfo> {
    if lines.is_empty() {
        return Err(ParaError::git_operation("Empty worktree block".to_string()));
    }

    // First line must be worktree path
    let first_line = lines[0];
    let path_str = first_line.strip_prefix("worktree ").ok_or_else(|| {
        ParaError::git_operation(format!("Invalid worktree block: {}", first_line))
    })?;

    let mut worktree = WorktreeInfo {
        path: PathBuf::from(path_str),
        branch: String::new(),
        commit: String::new(),
        is_bare: false,
    };

    // Process remaining lines
    for &line in &lines[1..] {
        parse_worktree_line(line, &mut worktree)?;
    }

    Ok(worktree)
}

fn parse_worktree_line(line: &str, worktree: &mut WorktreeInfo) -> Result<()> {
    if let Some(commit) = line.strip_prefix("HEAD ") {
        worktree.commit = commit.to_string();
    } else if let Some(branch) = line.strip_prefix("branch ") {
        let branch_name = branch.strip_prefix("refs/heads/").unwrap_or(branch);
        worktree.branch = branch_name.to_string();
    } else if line == "bare" {
        worktree.is_bare = true;
    } else if line == "detached" {
        worktree.branch = "HEAD".to_string();
    }

    Ok(())
}

impl<'a> WorktreeManager<'a> {
    pub fn validate_worktree(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(ParaError::git_operation(format!(
                "Worktree path does not exist: {}",
                path.display()
            )));
        }

        if !path.is_dir() {
            return Err(ParaError::git_operation(format!(
                "Worktree path is not a directory: {}",
                path.display()
            )));
        }

        let git_file = path.join(".git");
        if !git_file.exists() {
            return Err(ParaError::git_operation(format!(
                "Worktree is not properly configured (missing .git): {}",
                path.display()
            )));
        }

        GitRepository::discover_from(path)?;
        Ok(())
    }

    pub fn get_worktree_branch(&self, path: &Path) -> Result<String> {
        let temp_repo = GitRepository::discover_from(path)?;
        temp_repo.get_current_branch()
    }

    pub fn prune_worktrees(&self) -> Result<()> {
        execute_git_command_with_status(self.repo, &["worktree", "prune"])
    }

    pub fn is_worktree_path(&self, path: &Path) -> bool {
        GitRepository::discover_from(path)
            .map(|discovered_repo| discovered_repo.root != self.repo.root)
            .unwrap_or(false)
    }

    pub fn cleanup_stale_worktrees(&self) -> Result<Vec<PathBuf>> {
        let mut cleaned_paths = Vec::new();
        let worktrees = self.list_worktrees()?;

        for worktree in worktrees {
            if !worktree.path.exists() || worktree.path == self.repo.root {
                continue;
            }

            if self.validate_worktree(&worktree.path).is_err() {
                match self.force_remove_worktree(&worktree.path) {
                    Ok(()) => cleaned_paths.push(worktree.path),
                    Err(_) => continue,
                }
            }
        }

        self.prune_worktrees()?;
        Ok(cleaned_paths)
    }

    fn validate_branch_name(&self, branch_name: &str) -> Result<()> {
        GitValidator::validate_branch_name(branch_name)
    }

    fn validate_worktree_path(&self, path: &Path) -> Result<()> {
        GitValidator::validate_worktree_path(path, &self.repo.root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use std::fs;

    #[test]
    fn test_create_and_remove_worktree() {
        let (temp_dir, git_service) = setup_test_repo();
        let manager = WorktreeManager::new(git_service.repository());

        let worktree_path = temp_dir.path().join("feature-worktree");

        manager
            .create_worktree("feature-branch", &worktree_path)
            .expect("Failed to create worktree");

        assert!(worktree_path.exists());
        assert!(manager.validate_worktree(&worktree_path).is_ok());

        let branch = manager
            .get_worktree_branch(&worktree_path)
            .expect("Failed to get worktree branch");
        assert_eq!(branch, "feature-branch");

        manager
            .remove_worktree(&worktree_path)
            .expect("Failed to remove worktree");

        assert!(!worktree_path.exists());
    }

    #[test]
    fn test_list_worktrees() {
        let (temp_dir, git_service) = setup_test_repo();
        let manager = WorktreeManager::new(git_service.repository());

        let worktrees = manager.list_worktrees().expect("Failed to list worktrees");
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, git_service.repository().root);

        let worktree_path = temp_dir.path().join("test-worktree");
        manager
            .create_worktree("test-branch", &worktree_path)
            .expect("Failed to create worktree");

        let worktrees = manager.list_worktrees().expect("Failed to list worktrees");
        assert_eq!(worktrees.len(), 2);

        let feature_worktree = worktrees
            .iter()
            .find(|w| w.path.canonicalize().unwrap() == worktree_path.canonicalize().unwrap())
            .expect("Feature worktree not found");
        assert_eq!(feature_worktree.branch, "test-branch");
    }

    #[test]
    fn test_find_worktree_by_branch() {
        let (temp_dir, git_service) = setup_test_repo();
        let manager = WorktreeManager::new(git_service.repository());

        let worktree_path = temp_dir.path().join("find-test");
        manager
            .create_worktree("find-branch", &worktree_path)
            .expect("Failed to create worktree");

        // Helper function to find worktree by branch name
        let find_worktree_by_branch = |branch_name: &str| -> Result<Option<PathBuf>> {
            let worktrees = manager.list_worktrees()?;
            for worktree in worktrees {
                if worktree.branch == branch_name {
                    return Ok(Some(worktree.path));
                }
            }
            Ok(None)
        };

        let found_path = find_worktree_by_branch("find-branch")
            .expect("Failed to find worktree")
            .expect("Worktree not found");

        assert_eq!(
            found_path.canonicalize().unwrap(),
            worktree_path.canonicalize().unwrap()
        );

        let not_found =
            find_worktree_by_branch("nonexistent-branch").expect("Failed to search for worktree");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_invalid_branch_names() {
        let (_temp_dir, git_service) = setup_test_repo();
        let manager = WorktreeManager::new(git_service.repository());

        let test_cases = vec!["", "branch..name", "-invalid", "invalid/", "branch name"];

        for invalid_name in test_cases {
            let result = manager.validate_branch_name(invalid_name);
            assert!(
                result.is_err(),
                "Should reject invalid branch name: {}",
                invalid_name
            );
        }
    }

    #[test]
    fn test_worktree_validation() {
        let (temp_dir, git_service) = setup_test_repo();
        let manager = WorktreeManager::new(git_service.repository());

        let nonexistent_path = temp_dir.path().join("nonexistent");
        assert!(manager.validate_worktree(&nonexistent_path).is_err());

        let file_path = temp_dir.path().join("not-a-dir");
        fs::write(&file_path, "content").expect("Failed to write file");
        assert!(manager.validate_worktree(&file_path).is_err());

        assert!(manager
            .validate_worktree(&git_service.repository().root)
            .is_ok());
    }

    #[test]
    fn test_is_worktree_path() {
        let (temp_dir, git_service) = setup_test_repo();
        let manager = WorktreeManager::new(git_service.repository());

        assert!(!manager.is_worktree_path(&git_service.repository().root));

        let worktree_path = temp_dir.path().join("worktree-test");
        manager
            .create_worktree("test-wt", &worktree_path)
            .expect("Failed to create worktree");

        assert!(manager.is_worktree_path(&worktree_path));
    }
}
