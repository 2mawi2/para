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
                ParaError::git_operation(format!("Failed to create parent directory: {e}"))
            })?;
        }

        let path_str = path.to_string_lossy();

        let branch_exists = execute_git_command(
            self.repo,
            &[
                "rev-parse",
                "--verify",
                &format!("refs/heads/{branch_name}"),
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
                ParaError::git_operation(format!("Failed to remove worktree directory: {e}"))
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
                ParaError::git_operation(format!("Failed to force remove worktree directory: {e}"))
            })?;
        }

        self.prune_worktrees()?;
        Ok(())
    }

    pub fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let output = execute_git_command(self.repo, &["worktree", "list", "--porcelain"])?;
        WorktreePorcelainParser::parse(&output)
    }
}

/// Dedicated parser for git worktree porcelain output
struct WorktreePorcelainParser {
    worktrees: Vec<WorktreeInfo>,
    current_worktree: Option<WorktreeInfo>,
}

impl WorktreePorcelainParser {
    fn new() -> Self {
        Self {
            worktrees: Vec::new(),
            current_worktree: None,
        }
    }

    fn parse(porcelain_output: &str) -> Result<Vec<WorktreeInfo>> {
        let mut parser = Self::new();
        for line in porcelain_output.lines() {
            parser.process_line(line.trim())?;
        }
        parser.finalize();
        Ok(parser.worktrees)
    }

    fn process_line(&mut self, line: &str) -> Result<()> {
        if line.is_empty() {
            self.finish_current_worktree();
        } else if line.starts_with("worktree ") {
            self.process_worktree_line(line)?;
        } else if line.starts_with("HEAD ") {
            self.process_head_line(line)?;
        } else if line.starts_with("branch ") {
            self.process_branch_line(line)?;
        } else if line == "bare" {
            self.process_bare_line()?;
        } else if line == "detached" {
            self.process_detached_line()?;
        }
        // Ignore unknown lines
        Ok(())
    }

    fn process_worktree_line(&mut self, line: &str) -> Result<()> {
        self.finish_current_worktree();

        let path_str = line
            .strip_prefix("worktree ")
            .ok_or_else(|| ParaError::git_operation(format!("Invalid worktree line: {line}")))?;

        self.current_worktree = Some(WorktreeInfo {
            path: PathBuf::from(path_str),
            branch: String::new(),
            commit: String::new(),
            is_bare: false,
        });

        Ok(())
    }

    fn process_head_line(&mut self, line: &str) -> Result<()> {
        if let Some(ref mut worktree) = self.current_worktree {
            if let Some(commit) = line.strip_prefix("HEAD ") {
                worktree.commit = commit.to_string();
            }
        }
        Ok(())
    }

    fn process_branch_line(&mut self, line: &str) -> Result<()> {
        if let Some(ref mut worktree) = self.current_worktree {
            if let Some(branch) = line.strip_prefix("branch ") {
                let branch_name = branch.strip_prefix("refs/heads/").unwrap_or(branch);
                worktree.branch = branch_name.to_string();
            }
        }
        Ok(())
    }

    fn process_bare_line(&mut self) -> Result<()> {
        if let Some(ref mut worktree) = self.current_worktree {
            worktree.is_bare = true;
        }
        Ok(())
    }

    fn process_detached_line(&mut self) -> Result<()> {
        if let Some(ref mut worktree) = self.current_worktree {
            worktree.branch = "HEAD".to_string();
        }
        Ok(())
    }

    fn finish_current_worktree(&mut self) {
        if let Some(worktree) = self.current_worktree.take() {
            self.worktrees.push(worktree);
        }
    }

    fn finalize(&mut self) {
        self.finish_current_worktree();
    }
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
                "Should reject invalid branch name: {invalid_name}"
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

    // Unit tests for the WorktreePorcelainParser

    // Parser-specific unit tests
    #[test]
    fn test_worktree_porcelain_parser_empty_input() {
        let result = WorktreePorcelainParser::parse("").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_worktree_porcelain_parser_whitespace_only() {
        let result = WorktreePorcelainParser::parse("   \n\n  \t\n   ").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_worktree_porcelain_parser_single_worktree() {
        let input = "worktree /path/to/repo\nHEAD abc123def456\nbranch refs/heads/main";
        let worktrees = WorktreePorcelainParser::parse(input).unwrap();

        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, PathBuf::from("/path/to/repo"));
        assert_eq!(worktrees[0].commit, "abc123def456");
        assert_eq!(worktrees[0].branch, "main");
        assert!(!worktrees[0].is_bare);
    }

    #[test]
    fn test_worktree_porcelain_parser_multiple_worktrees() {
        let input = r#"worktree /main
HEAD abc123
branch refs/heads/main

worktree /feature
HEAD def456
branch feature-branch

worktree /bare.git
HEAD 789abc
bare"#;
        let worktrees = WorktreePorcelainParser::parse(input).unwrap();

        assert_eq!(worktrees.len(), 3);

        assert_eq!(worktrees[0].path, PathBuf::from("/main"));
        assert_eq!(worktrees[0].branch, "main");
        assert!(!worktrees[0].is_bare);

        assert_eq!(worktrees[1].path, PathBuf::from("/feature"));
        assert_eq!(worktrees[1].branch, "feature-branch");
        assert!(!worktrees[1].is_bare);

        assert_eq!(worktrees[2].path, PathBuf::from("/bare.git"));
        assert_eq!(worktrees[2].branch, "");
        assert!(worktrees[2].is_bare);
    }

    #[test]
    fn test_worktree_porcelain_parser_detached_head() {
        let input = "worktree /detached\nHEAD 987654321def\ndetached";
        let worktrees = WorktreePorcelainParser::parse(input).unwrap();

        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, PathBuf::from("/detached"));
        assert_eq!(worktrees[0].commit, "987654321def");
        assert_eq!(worktrees[0].branch, "HEAD");
        assert!(!worktrees[0].is_bare);
    }

    #[test]
    fn test_worktree_porcelain_parser_branch_without_prefix() {
        let input = "worktree /test\nHEAD abc123\nbranch feature-no-prefix";
        let worktrees = WorktreePorcelainParser::parse(input).unwrap();

        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].branch, "feature-no-prefix");
    }

    #[test]
    fn test_worktree_porcelain_parser_minimal_worktree() {
        let input = "worktree /minimal";
        let worktrees = WorktreePorcelainParser::parse(input).unwrap();

        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, PathBuf::from("/minimal"));
        assert_eq!(worktrees[0].commit, "");
        assert_eq!(worktrees[0].branch, "");
        assert!(!worktrees[0].is_bare);
    }

    #[test]
    fn test_worktree_porcelain_parser_unknown_lines_ignored() {
        let input = r#"worktree /test
unknown-line-1
HEAD abc123
unknown-line-2
branch main
unknown-line-3"#;
        let worktrees = WorktreePorcelainParser::parse(input).unwrap();

        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, PathBuf::from("/test"));
        assert_eq!(worktrees[0].commit, "abc123");
        assert_eq!(worktrees[0].branch, "main");
    }

    #[test]
    fn test_worktree_porcelain_parser_extra_whitespace() {
        let input = r#"
  
worktree /test  
  HEAD abc123  
  branch main  
  

worktree /test2
HEAD def456
branch feature


  "#;
        let worktrees = WorktreePorcelainParser::parse(input).unwrap();

        assert_eq!(worktrees.len(), 2);
        assert_eq!(worktrees[0].path, PathBuf::from("/test"));
        assert_eq!(worktrees[0].commit, "abc123");
        assert_eq!(worktrees[0].branch, "main");

        assert_eq!(worktrees[1].path, PathBuf::from("/test2"));
        assert_eq!(worktrees[1].commit, "def456");
        assert_eq!(worktrees[1].branch, "feature");
    }

    #[test]
    fn test_worktree_porcelain_parser_invalid_worktree_line() {
        let input = "invalid-worktree-line\nHEAD abc123";
        let result = WorktreePorcelainParser::parse(input);
        // Should not error, just ignore invalid lines
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_worktree_porcelain_parser_mixed_valid_invalid() {
        let input = r#"invalid-line
some-garbage

worktree /valid
HEAD abc123
branch main

more-garbage
invalid-line

worktree /valid2
HEAD def456
branch feature"#;
        let worktrees = WorktreePorcelainParser::parse(input).unwrap();

        assert_eq!(worktrees.len(), 2);
        assert_eq!(worktrees[0].path, PathBuf::from("/valid"));
        assert_eq!(worktrees[0].branch, "main");
        assert_eq!(worktrees[1].path, PathBuf::from("/valid2"));
        assert_eq!(worktrees[1].branch, "feature");
    }

    #[test]
    fn test_worktree_porcelain_parser_all_line_types() {
        let input = r#"worktree /comprehensive-test
HEAD 1234567890abcdef
branch refs/heads/feature-branch
bare
detached"#;
        let worktrees = WorktreePorcelainParser::parse(input).unwrap();

        assert_eq!(worktrees.len(), 1);
        let worktree = &worktrees[0];
        assert_eq!(worktree.path, PathBuf::from("/comprehensive-test"));
        assert_eq!(worktree.commit, "1234567890abcdef");
        // Last value wins for branch (detached overwrites feature-branch)
        assert_eq!(worktree.branch, "HEAD");
        assert!(worktree.is_bare);
    }
}
