use crate::utils::error::{ParaError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitRepository {
    pub root: PathBuf,
    pub git_dir: PathBuf,
    pub work_dir: PathBuf,
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
        let work_dir = root.clone();

        Ok(Self { root, git_dir, work_dir })
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
        let default_branch = execute_git_command(self, &["symbolic-ref", "refs/remotes/origin/HEAD"]);
        if let Ok(branch_ref) = default_branch {
            if let Some(branch_name) = branch_ref.strip_prefix("refs/remotes/origin/") {
                return Ok(branch_name.to_string());
            }
        }

        let branches = ["main", "master", "develop"];
        for branch in &branches {
            if execute_git_command(self, &["show-ref", "--verify", "--quiet", &format!("refs/heads/{}", branch)]).is_ok() {
                return Ok(branch.to_string());
            }
        }

        Ok("main".to_string())
    }

    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        let output = execute_git_command(self, &["status", "--porcelain"])?;
        Ok(!output.trim().is_empty())
    }

    pub fn get_commit_count_since(&self, base_branch: &str, feature_branch: &str) -> Result<usize> {
        let range = format!("{}..{}", base_branch, feature_branch);
        let output = execute_git_command(self, &["rev-list", "--count", &range])?;

        output
            .trim()
            .parse::<usize>()
            .map_err(|e| ParaError::git_operation(format!("Failed to parse commit count: {}", e)))
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

    pub fn get_remote_url(&self) -> Result<Option<String>> {
        match execute_git_command(self, &["remote", "get-url", "origin"]) {
            Ok(url) => Ok(Some(url.trim().to_string())),
            Err(_) => Ok(None),
        }
    }

    pub fn get_merge_base(&self, branch1: &str, branch2: &str) -> Result<String> {
        execute_git_command(self, &["merge-base", branch1, branch2])
    }

    pub fn get_head_commit(&self) -> Result<String> {
        execute_git_command(self, &["rev-parse", "HEAD"])
    }

    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool> {
        let result = Command::new("git")
            .current_dir(&self.root)
            .args(["merge-base", "--is-ancestor", ancestor, descendant])
            .status()
            .map_err(|e| ParaError::git_operation(format!("Failed to check ancestry: {}", e)))?;

        Ok(result.success())
    }

    pub fn stage_all_changes(&self) -> Result<()> {
        execute_git_command_with_status(self, &["add", "."])
    }

    pub fn commit(&self, message: &str) -> Result<()> {
        let sanitized_message = sanitize_commit_message(message);
        execute_git_command_with_status(self, &["commit", "-m", &sanitized_message])
    }

    pub fn checkout_branch(&self, branch: &str) -> Result<()> {
        execute_git_command_with_status(self, &["checkout", branch])
    }

    pub fn reset_hard(&self, commit: &str) -> Result<()> {
        execute_git_command_with_status(self, &["reset", "--hard", commit])
    }

    pub fn get_commit_message(&self, commit: &str) -> Result<String> {
        execute_git_command(self, &["log", "--format=%B", "-n", "1", commit])
    }

    pub fn merge_fast_forward(&self, branch: &str) -> Result<()> {
        execute_git_command_with_status(self, &["merge", "--ff-only", branch])
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
            .args(&["init"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["commit", "-m", "Initial commit"])
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
        assert!(branch == "main" || branch == "master");
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
}
