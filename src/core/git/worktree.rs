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

    // Unit tests for the new parsing functions

    #[test]
    fn test_parse_worktree_line() {
        let mut worktree = WorktreeInfo {
            path: PathBuf::from("/test/path"),
            branch: String::new(),
            commit: String::new(),
            is_bare: false,
        };

        // Test HEAD line
        parse_worktree_line("HEAD abc123def456", &mut worktree).unwrap();
        assert_eq!(worktree.commit, "abc123def456");

        // Test branch line with refs/heads/ prefix
        parse_worktree_line("branch refs/heads/main", &mut worktree).unwrap();
        assert_eq!(worktree.branch, "main");

        // Test branch line without refs/heads/ prefix
        worktree.branch.clear();
        parse_worktree_line("branch feature-branch", &mut worktree).unwrap();
        assert_eq!(worktree.branch, "feature-branch");

        // Test bare line
        parse_worktree_line("bare", &mut worktree).unwrap();
        assert!(worktree.is_bare);

        // Test detached line
        worktree.branch.clear();
        parse_worktree_line("detached", &mut worktree).unwrap();
        assert_eq!(worktree.branch, "HEAD");

        // Test unknown line (should not error or modify anything)
        let original_commit = worktree.commit.clone();
        let original_branch = worktree.branch.clone();
        let original_bare = worktree.is_bare;
        parse_worktree_line("unknown-line-type something", &mut worktree).unwrap();
        assert_eq!(worktree.commit, original_commit);
        assert_eq!(worktree.branch, original_branch);
        assert_eq!(worktree.is_bare, original_bare);
    }

    #[test]
    fn test_parse_worktree_block_main_worktree() {
        let lines = vec![
            "worktree /path/to/repo",
            "HEAD abc123def456",
            "branch refs/heads/main",
        ];
        let worktree = parse_worktree_block(&lines).unwrap();

        assert_eq!(worktree.path, PathBuf::from("/path/to/repo"));
        assert_eq!(worktree.commit, "abc123def456");
        assert_eq!(worktree.branch, "main");
        assert!(!worktree.is_bare);
    }

    #[test]
    fn test_parse_worktree_block_feature_branch() {
        let lines = vec![
            "worktree /path/to/feature",
            "HEAD def456abc123",
            "branch feature-branch",
        ];
        let worktree = parse_worktree_block(&lines).unwrap();

        assert_eq!(worktree.path, PathBuf::from("/path/to/feature"));
        assert_eq!(worktree.commit, "def456abc123");
        assert_eq!(worktree.branch, "feature-branch");
        assert!(!worktree.is_bare);
    }

    #[test]
    fn test_parse_worktree_block_bare() {
        let lines = vec!["worktree /path/to/bare.git", "HEAD 123456789abc", "bare"];
        let worktree = parse_worktree_block(&lines).unwrap();

        assert_eq!(worktree.path, PathBuf::from("/path/to/bare.git"));
        assert_eq!(worktree.commit, "123456789abc");
        assert_eq!(worktree.branch, "");
        assert!(worktree.is_bare);
    }

    #[test]
    fn test_parse_worktree_block_detached() {
        let lines = vec![
            "worktree /path/to/detached",
            "HEAD 987654321def",
            "detached",
        ];
        let worktree = parse_worktree_block(&lines).unwrap();

        assert_eq!(worktree.path, PathBuf::from("/path/to/detached"));
        assert_eq!(worktree.commit, "987654321def");
        assert_eq!(worktree.branch, "HEAD");
        assert!(!worktree.is_bare);
    }

    #[test]
    fn test_parse_worktree_block_empty() {
        let lines: Vec<&str> = vec![];
        let result = parse_worktree_block(&lines);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Empty worktree block"));
    }

    #[test]
    fn test_parse_worktree_block_invalid_first_line() {
        let lines = vec!["invalid-first-line", "HEAD abc123def456"];
        let result = parse_worktree_block(&lines);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid worktree block"));
    }

    #[test]
    fn test_parse_worktree_block_minimal() {
        let lines = vec!["worktree /minimal/path"];
        let worktree = parse_worktree_block(&lines).unwrap();

        assert_eq!(worktree.path, PathBuf::from("/minimal/path"));
        assert_eq!(worktree.commit, "");
        assert_eq!(worktree.branch, "");
        assert!(!worktree.is_bare);
    }

    #[test]
    fn test_parse_worktree_output_single() {
        let output = "worktree /path/to/repo\nHEAD abc123def456\nbranch refs/heads/main\n";
        let worktrees = parse_worktree_output(output).unwrap();

        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, PathBuf::from("/path/to/repo"));
        assert_eq!(worktrees[0].commit, "abc123def456");
        assert_eq!(worktrees[0].branch, "main");
        assert!(!worktrees[0].is_bare);
    }

    #[test]
    fn test_parse_worktree_output_multiple() {
        let output = r#"worktree /path/to/repo
HEAD abc123def456
branch refs/heads/main

worktree /path/to/feature
HEAD def456abc123
branch feature-branch

worktree /path/to/bare.git
HEAD 123456789abc
bare
"#;
        let worktrees = parse_worktree_output(output).unwrap();

        assert_eq!(worktrees.len(), 3);

        // Main worktree
        assert_eq!(worktrees[0].path, PathBuf::from("/path/to/repo"));
        assert_eq!(worktrees[0].branch, "main");
        assert!(!worktrees[0].is_bare);

        // Feature worktree
        assert_eq!(worktrees[1].path, PathBuf::from("/path/to/feature"));
        assert_eq!(worktrees[1].branch, "feature-branch");
        assert!(!worktrees[1].is_bare);

        // Bare worktree
        assert_eq!(worktrees[2].path, PathBuf::from("/path/to/bare.git"));
        assert_eq!(worktrees[2].branch, "");
        assert!(worktrees[2].is_bare);
    }

    #[test]
    fn test_parse_worktree_output_empty() {
        let output = "";
        let worktrees = parse_worktree_output(output).unwrap();
        assert_eq!(worktrees.len(), 0);
    }

    #[test]
    fn test_parse_worktree_output_whitespace_only() {
        let output = "   \n\n  \t\n   ";
        let worktrees = parse_worktree_output(output).unwrap();
        assert_eq!(worktrees.len(), 0);
    }

    #[test]
    fn test_parse_worktree_output_with_extra_empty_lines() {
        let output = r#"

worktree /path/to/repo
HEAD abc123def456
branch refs/heads/main



worktree /path/to/feature
HEAD def456abc123
branch feature-branch


"#;
        let worktrees = parse_worktree_output(output).unwrap();

        assert_eq!(worktrees.len(), 2);
        assert_eq!(worktrees[0].path, PathBuf::from("/path/to/repo"));
        assert_eq!(worktrees[1].path, PathBuf::from("/path/to/feature"));
    }

    #[test]
    fn test_parse_worktree_output_mixed_scenarios() {
        let output = r#"worktree /path/to/main
HEAD abc123def456
branch refs/heads/main

worktree /path/to/detached
HEAD 987654321def
detached

worktree /path/to/feature
HEAD def456abc123
branch feature-without-prefix

worktree /path/to/bare.git
HEAD 123456789abc
bare
"#;
        let worktrees = parse_worktree_output(output).unwrap();

        assert_eq!(worktrees.len(), 4);

        // Main branch
        assert_eq!(worktrees[0].branch, "main");
        assert!(!worktrees[0].is_bare);

        // Detached HEAD
        assert_eq!(worktrees[1].branch, "HEAD");
        assert!(!worktrees[1].is_bare);

        // Feature branch without refs/heads/
        assert_eq!(worktrees[2].branch, "feature-without-prefix");
        assert!(!worktrees[2].is_bare);

        // Bare repository
        assert_eq!(worktrees[3].branch, "");
        assert!(worktrees[3].is_bare);
    }

    #[test]
    fn test_parse_worktree_output_invalid_blocks_skipped() {
        let output = r#"invalid-block-without-worktree-prefix
HEAD abc123def456

worktree /valid/path
HEAD def456abc123
branch valid-branch

another-invalid-block
some-content
"#;
        let worktrees = parse_worktree_output(output).unwrap();

        // Should only parse the valid block and skip invalid ones
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, PathBuf::from("/valid/path"));
        assert_eq!(worktrees[0].branch, "valid-branch");
    }
}
