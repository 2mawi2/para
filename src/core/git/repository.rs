use crate::utils::error::{ParaError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitRepository {
    pub root: PathBuf,
    pub git_dir: PathBuf,
}

impl GitRepository {
    pub fn discover() -> Result<Self> {
        let current_dir = std::env::current_dir().map_err(|e| {
            ParaError::git_operation(format!("Failed to get current directory: {}", e))
        })?;

        Self::discover_from(&current_dir)
    }

    pub fn discover_from(path: &Path) -> Result<Self> {
        let output = Command::new("git")
            .current_dir(path)
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .map_err(|e| ParaError::git_operation(format!("Failed to execute git: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ParaError::git_operation(format!(
                "Not a git repository or git not found: {}",
                stderr.trim()
            )));
        }

        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let root = PathBuf::from(root);

        let git_dir = Self::get_git_dir(&root)?;

        Ok(Self { root, git_dir })
    }

    pub fn validate(&self) -> Result<()> {
        if !self.root.exists() {
            return Err(ParaError::git_operation(
                "Repository root does not exist".to_string(),
            ));
        }

        if !self.git_dir.exists() {
            return Err(ParaError::git_operation(
                "Git directory does not exist".to_string(),
            ));
        }

        let output = Command::new("git")
            .current_dir(&self.root)
            .args(["status", "--porcelain"])
            .output()
            .map_err(|e| ParaError::git_operation(format!("Failed to check git status: {}", e)))?;

        if !output.status.success() {
            return Err(ParaError::git_operation(
                "Repository is in an invalid state".to_string(),
            ));
        }

        Ok(())
    }

    pub fn get_current_branch(&self) -> Result<String> {
        execute_git_command(self, &["rev-parse", "--abbrev-ref", "HEAD"])
    }

    pub fn get_main_branch(&self) -> Result<String> {
        // Always prefer a local 'main' branch if it exists to encourage modern default.
        if execute_git_command(
            self,
            &["show-ref", "--verify", "--quiet", "refs/heads/main"],
        )
        .is_ok()
        {
            return Ok("main".to_string());
        }

        // Fallback to remote HEAD reference (could be master or something else).
        if let Ok(branch_ref) =
            execute_git_command(self, &["symbolic-ref", "refs/remotes/origin/HEAD"])
        {
            if let Some(branch_name) = branch_ref.strip_prefix("refs/remotes/origin/") {
                return Ok(branch_name.to_string());
            }
        }

        // Legacy repositories might use 'master'. Detect it explicitly.
        if execute_git_command(
            self,
            &["show-ref", "--verify", "--quiet", "refs/heads/master"],
        )
        .is_ok()
        {
            return Ok("master".to_string());
        }

        // As another common pattern, check for 'develop'.
        if execute_git_command(
            self,
            &["show-ref", "--verify", "--quiet", "refs/heads/develop"],
        )
        .is_ok()
        {
            return Ok("develop".to_string());
        }

        // Default backstop.
        Ok("main".to_string())
    }

    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        let output = execute_git_command(self, &["status", "--porcelain"])?;
        Ok(!output.trim().is_empty())
    }

    pub fn is_clean_working_tree(&self) -> Result<bool> {
        let status_output = execute_git_command(self, &["status", "--porcelain"])?;
        let has_staged = !status_output.trim().is_empty();

        if has_staged {
            return Ok(false);
        }

        let diff_output = execute_git_command(self, &["diff", "--quiet"]);
        Ok(diff_output.is_ok())
    }

    pub fn stage_all_changes(&self) -> Result<()> {
        execute_git_command_with_status(self, &["add", "."]).map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("embedded repository")
                || error_str.contains("adding an embedded repository")
                || error_str.contains("does not have a commit checked out")
                || error_str.contains("adding files failed")
            {
                ParaError::git_operation(format!(
                    "Cannot stage files due to nested git repositories in worktree.\n\n\
                    This usually indicates:\n\
                    1. Test artifacts weren't cleaned up properly\n\
                    2. Nested git repositories in your worktree\n\n\
                    Solutions:\n\
                    • Run 'git status' to see problematic directories\n\
                    • Remove unwanted nested repositories manually\n\
                    • Add them to .gitignore if they should be ignored\n\n\
                    Note: Para doesn't support worktrees with nested git repositories.\n\
                    Original error: {}",
                    error_str
                ))
            } else {
                e
            }
        })
    }

    pub fn commit(&self, message: &str) -> Result<()> {
        let sanitized_message = sanitize_commit_message(message);
        execute_git_command_with_status(self, &["commit", "-m", &sanitized_message])
    }

    pub fn checkout_branch(&self, branch: &str) -> Result<()> {
        execute_git_command_with_status(self, &["checkout", branch])
    }

    fn get_git_dir(repo_root: &Path) -> Result<PathBuf> {
        let output = Command::new("git")
            .current_dir(repo_root)
            .args(["rev-parse", "--git-dir"])
            .output()
            .map_err(|e| ParaError::git_operation(format!("Failed to get git dir: {}", e)))?;

        if !output.status.success() {
            return Err(ParaError::git_operation(
                "Failed to determine git directory".to_string(),
            ));
        }

        let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let git_dir = if git_dir.starts_with('/') {
            PathBuf::from(git_dir)
        } else {
            repo_root.join(git_dir)
        };

        Ok(git_dir)
    }
}

pub fn execute_git_command(repo: &GitRepository, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(&repo.root)
        .args(args)
        .output()
        .map_err(|e| ParaError::git_operation(format!("Failed to execute git: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ParaError::git_operation(format!(
            "Git command failed ({}): {}",
            args.join(" "),
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

pub fn execute_git_command_with_status(repo: &GitRepository, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .current_dir(&repo.root)
        .args(args)
        .status()
        .map_err(|e| ParaError::git_operation(format!("Failed to execute git: {}", e)))?;

    if !status.success() {
        return Err(ParaError::git_operation(format!(
            "Git command failed: {}",
            args.join(" ")
        )));
    }

    Ok(())
}

fn sanitize_commit_message(message: &str) -> String {
    message
        .lines()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, GitRepository) {
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

        let repo = GitRepository::discover_from(repo_path).expect("Failed to discover repo");
        (temp_dir, repo)
    }

    #[test]
    fn test_repository_discovery() {
        let (temp_dir, repo) = setup_test_repo();
        assert_eq!(repo.root, temp_dir.path().canonicalize().unwrap());
        assert!(repo.git_dir.exists());
    }

    #[test]
    fn test_repository_validation() {
        let (_temp_dir, repo) = setup_test_repo();
        assert!(repo.validate().is_ok());
    }

    #[test]
    fn test_get_current_branch() {
        let (_temp_dir, repo) = setup_test_repo();
        let branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");
        assert!(branch == "main");
    }

    #[test]
    fn test_clean_working_tree() {
        let (_temp_dir, repo) = setup_test_repo();
        assert!(repo
            .is_clean_working_tree()
            .expect("Failed to check clean state"));
    }

    #[test]
    fn test_sanitize_commit_message() {
        let message = "  Test commit  \n  with multiple lines  \n  ";
        let sanitized = sanitize_commit_message(message);
        assert_eq!(sanitized, "Test commit\nwith multiple lines");
    }

    #[test]
    fn test_has_uncommitted_changes() {
        let (temp_dir, repo) = setup_test_repo();

        assert!(!repo
            .has_uncommitted_changes()
            .expect("Failed to check changes"));

        fs::write(temp_dir.path().join("test.txt"), "test content")
            .expect("Failed to write test file");

        assert!(repo
            .has_uncommitted_changes()
            .expect("Failed to check changes"));
    }

    #[test]
    fn test_stage_all_changes_with_nested_repo() {
        let (temp_dir, repo) = setup_test_repo();

        // Create a normal file
        fs::write(temp_dir.path().join("normal.txt"), "normal content")
            .expect("Failed to write normal file");

        // Create a nested git repository (test artifact)
        let nested_repo_path = temp_dir.path().join("test-nested-repo");
        fs::create_dir_all(&nested_repo_path).expect("Failed to create nested dir");

        Command::new("git")
            .current_dir(&nested_repo_path)
            .args(["init"])
            .status()
            .expect("Failed to init nested repo");

        // Try to stage all changes - should fail with clear error
        let result = repo.stage_all_changes();
        assert!(result.is_err());

        let error_message = result.unwrap_err().to_string();
        eprintln!("Error message: {}", error_message);

        // Check for either the git error or our custom message
        assert!(
            error_message.contains("nested git repositories")
                || error_message.contains("embedded repository")
                || error_message.contains("Git command failed"),
            "Expected error about nested repositories, got: {}",
            error_message
        );

        // Clean up the nested repo
        fs::remove_dir_all(&nested_repo_path).expect("Failed to remove nested repo");

        // Now staging should work
        assert!(repo.stage_all_changes().is_ok());
    }

    #[test]
    fn test_stage_all_changes_normal_operation() {
        let (temp_dir, repo) = setup_test_repo();

        // Create some normal files
        fs::write(temp_dir.path().join("file1.txt"), "content1").expect("Failed to write file1");
        fs::write(temp_dir.path().join("file2.txt"), "content2").expect("Failed to write file2");

        // Staging should work normally
        assert!(repo.stage_all_changes().is_ok());

        // Verify files are staged
        let status =
            execute_git_command(&repo, &["status", "--porcelain"]).expect("Failed to get status");
        assert!(status.contains("A  file1.txt"));
        assert!(status.contains("A  file2.txt"));
    }
}
