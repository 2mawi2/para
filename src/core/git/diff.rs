use crate::core::status::DiffStats;
use crate::utils::error::{ParaError, Result};
use std::path::Path;
use std::process::Command;

/// Calculate git diff statistics between the current HEAD and a base branch
pub fn calculate_diff_stats(worktree_path: &Path, base_branch: &str) -> Result<DiffStats> {
    // First check if the path exists
    if !worktree_path.exists() {
        return Err(ParaError::git_operation(
            "Worktree path does not exist".to_string(),
        ));
    }

    // Check if we're in a git repository
    let status_output = Command::new("git")
        .current_dir(worktree_path)
        .args(["status", "--porcelain"])
        .output();

    match status_output {
        Ok(output) if output.status.success() => {} // Continue
        Ok(_) => {
            return Err(ParaError::git_operation(
                "Not in a git repository".to_string(),
            ));
        }
        Err(e) => {
            return Err(ParaError::git_operation(format!(
                "Git command failed: {}",
                e
            )));
        }
    }

    // Check if base branch exists
    let branch_check = Command::new("git")
        .current_dir(worktree_path)
        .args(["rev-parse", "--verify", base_branch])
        .output()
        .map_err(|e| {
            ParaError::git_operation(format!("Failed to check branch existence: {}", e))
        })?;

    if !branch_check.status.success() {
        return Err(ParaError::git_operation(format!(
            "Base branch '{}' does not exist",
            base_branch
        )));
    }

    // Get diff stats using git diff --numstat
    // Use two-dot notation (..) to show changes from base branch to current HEAD
    let output = Command::new("git")
        .current_dir(worktree_path)
        .args(["diff", "--numstat", &format!("{}..HEAD", base_branch)])
        .output()
        .map_err(|e| ParaError::git_operation(format!("Failed to get diff stats: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check if this is a detached HEAD state
        if stderr.contains("ambiguous argument 'HEAD'") {
            // Try using the current branch name instead
            let branch_output = Command::new("git")
                .current_dir(worktree_path)
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .output();

            if let Ok(output) = branch_output {
                if output.status.success() {
                    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if branch != "HEAD" {
                        // Retry with branch name
                        return calculate_diff_stats_with_branch(
                            worktree_path,
                            base_branch,
                            &branch,
                        );
                    }
                }
            }
        }
        return Err(ParaError::git_operation(format!(
            "Failed to calculate diff: {}",
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut additions = 0;
    let mut deletions = 0;

    // Parse the numstat output
    // Format: additions<TAB>deletions<TAB>filename
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            if let Ok(add) = parts[0].parse::<usize>() {
                additions += add;
            }
            if let Ok(del) = parts[1].parse::<usize>() {
                deletions += del;
            }
        }
    }

    // Also include unstaged changes
    let unstaged_output = Command::new("git")
        .current_dir(worktree_path)
        .args(["diff", "--numstat"])
        .output()
        .map_err(|e| {
            ParaError::git_operation(format!("Failed to get unstaged diff stats: {}", e))
        })?;

    if unstaged_output.status.success() {
        let unstaged_stdout = String::from_utf8_lossy(&unstaged_output.stdout);
        for line in unstaged_stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                if let Ok(add) = parts[0].parse::<usize>() {
                    additions += add;
                }
                if let Ok(del) = parts[1].parse::<usize>() {
                    deletions += del;
                }
            }
        }
    }

    // Also include staged changes
    let staged_output = Command::new("git")
        .current_dir(worktree_path)
        .args(["diff", "--cached", "--numstat"])
        .output()
        .map_err(|e| ParaError::git_operation(format!("Failed to get staged diff stats: {}", e)))?;

    if staged_output.status.success() {
        let staged_stdout = String::from_utf8_lossy(&staged_output.stdout);
        for line in staged_stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                if let Ok(add) = parts[0].parse::<usize>() {
                    additions += add;
                }
                if let Ok(del) = parts[1].parse::<usize>() {
                    deletions += del;
                }
            }
        }
    }

    Ok(DiffStats::new(additions, deletions))
}

fn calculate_diff_stats_with_branch(
    worktree_path: &Path,
    base_branch: &str,
    current_branch: &str,
) -> Result<DiffStats> {
    let output = Command::new("git")
        .current_dir(worktree_path)
        .args([
            "diff",
            "--numstat",
            &format!("{}..{}", base_branch, current_branch),
        ])
        .output()
        .map_err(|e| ParaError::git_operation(format!("Failed to get diff stats: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ParaError::git_operation(format!(
            "Failed to calculate diff: {}",
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut additions = 0;
    let mut deletions = 0;

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            if let Ok(add) = parts[0].parse::<usize>() {
                additions += add;
            }
            if let Ok(del) = parts[1].parse::<usize>() {
                deletions += del;
            }
        }
    }

    Ok(DiffStats::new(additions, deletions))
}

/// Try to find the parent branch for a given branch
pub fn find_parent_branch(worktree_path: &Path, current_branch: &str) -> Result<String> {
    // Special case: if current branch is main/master, use it as parent
    if current_branch == "main" || current_branch == "master" {
        return Ok(current_branch.to_string());
    }

    // Try to find the merge base with common default branches
    let common_branches = ["main", "master", "develop"];

    for candidate in &common_branches {
        let check = Command::new("git")
            .current_dir(worktree_path)
            .args([
                "rev-parse",
                "--verify",
                &format!("refs/heads/{}", candidate),
            ])
            .output();

        if let Ok(output) = check {
            if output.status.success() {
                // Check if this could be the parent by finding merge-base
                let merge_base = Command::new("git")
                    .current_dir(worktree_path)
                    .args(["merge-base", candidate, current_branch])
                    .output();

                if let Ok(mb_output) = merge_base {
                    if mb_output.status.success() {
                        return Ok(candidate.to_string());
                    }
                }
            }
        }
    }

    // Try to detect the parent from the upstream branch
    let upstream_output = Command::new("git")
        .current_dir(worktree_path)
        .args([
            "rev-parse",
            "--abbrev-ref",
            &format!("{}@{{upstream}}", current_branch),
        ])
        .output();

    if let Ok(output) = upstream_output {
        if output.status.success() {
            let upstream = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // Extract the branch name from origin/branch format
            if let Some(branch) = upstream.split('/').next_back() {
                return Ok(branch.to_string());
            }
        }
    }

    // If no common branch found, default to main
    Ok("main".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::git::GitOperations;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_calculate_diff_stats_no_changes() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let stats = calculate_diff_stats(git_temp.path(), "main").unwrap();
        assert_eq!(stats.additions, 0);
        assert_eq!(stats.deletions, 0);
    }

    #[test]
    fn test_calculate_diff_stats_with_changes() {
        let (git_temp, git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Create a new branch
        // Create and checkout a new branch
        git_service.create_branch("test-branch", "main").unwrap();

        // Manually checkout the branch since checkout_branch is not in GitOperations
        std::process::Command::new("git")
            .current_dir(git_temp.path())
            .args(["checkout", "test-branch"])
            .output()
            .unwrap();

        // Add a file with some content
        let test_file = git_temp.path().join("test.txt");
        std::fs::write(&test_file, "line 1\nline 2\nline 3\n").unwrap();

        // Stage and commit
        git_service.stage_all_changes().unwrap();

        // Manually commit since commit is not in GitOperations
        std::process::Command::new("git")
            .current_dir(git_temp.path())
            .args(["commit", "-m", "Add test file"])
            .output()
            .unwrap();

        let stats = calculate_diff_stats(git_temp.path(), "main").unwrap();
        assert_eq!(stats.additions, 3);
        assert_eq!(stats.deletions, 0);
    }

    #[test]
    fn test_calculate_diff_stats_not_git_repo() {
        let temp_dir = TempDir::new().unwrap();
        let result = calculate_diff_stats(temp_dir.path(), "main");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Not in a git repository"));
    }

    #[test]
    fn test_calculate_diff_stats_nonexistent_branch() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let result = calculate_diff_stats(git_temp.path(), "nonexistent-branch");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_find_parent_branch() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Should find main as the parent
        let parent = find_parent_branch(git_temp.path(), "main").unwrap();
        assert_eq!(parent, "main");
    }

    #[test]
    fn test_diff_stats_display() {
        let stats = DiffStats::new(123, 45);
        assert_eq!(stats.to_string(), "+123 -45");

        let stats_zero = DiffStats::new(0, 0);
        assert_eq!(stats_zero.to_string(), "+0 -0");
    }

    #[test]
    fn test_calculate_diff_stats_with_modifications() {
        let (git_temp, git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Create a new branch
        git_service.create_branch("feature-branch", "main").unwrap();

        // Checkout the branch
        std::process::Command::new("git")
            .current_dir(git_temp.path())
            .args(["checkout", "feature-branch"])
            .output()
            .unwrap();

        // Modify existing file
        let readme = git_temp.path().join("README.md");
        std::fs::write(&readme, "# Test Repo\nModified line\nNew line\n").unwrap();

        // Add a new file
        let new_file = git_temp.path().join("new_file.txt");
        std::fs::write(&new_file, "This is a new file\nWith multiple lines\n").unwrap();

        // Stage and commit
        git_service.stage_all_changes().unwrap();
        std::process::Command::new("git")
            .current_dir(git_temp.path())
            .args(["commit", "-m", "Add modifications"])
            .output()
            .unwrap();

        let stats = calculate_diff_stats(git_temp.path(), "main").unwrap();
        assert!(stats.additions > 0);
        // deletions could be 0 or more depending on what was modified
    }

    #[test]
    fn test_calculate_diff_stats_with_unstaged_changes() {
        let (git_temp, git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Create a branch to work on
        git_service.create_branch("work-branch", "main").unwrap();
        std::process::Command::new("git")
            .current_dir(git_temp.path())
            .args(["checkout", "work-branch"])
            .output()
            .unwrap();

        // Create a new file to have a clean test
        let new_file = git_temp.path().join("test.txt");
        std::fs::write(&new_file, "Line 1\nLine 2\nLine 3\n").unwrap();

        // Add and commit the file
        git_service.stage_all_changes().unwrap();
        std::process::Command::new("git")
            .current_dir(git_temp.path())
            .args(["commit", "-m", "Add test file"])
            .output()
            .unwrap();

        // Now append to create unstaged changes
        let content = std::fs::read_to_string(&new_file).unwrap();
        std::fs::write(&new_file, format!("{}Line 4\n", content)).unwrap();

        let stats = calculate_diff_stats(git_temp.path(), "main").unwrap();
        // We committed a file with 3 lines and added 1 more line as unstaged
        // So total should be 4 additions compared to main
        assert_eq!(stats.additions, 4);
        assert_eq!(stats.deletions, 0);
    }

    #[test]
    fn test_calculate_diff_stats_with_staged_changes() {
        let (git_temp, git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Create a branch to work on
        git_service.create_branch("stage-branch", "main").unwrap();
        std::process::Command::new("git")
            .current_dir(git_temp.path())
            .args(["checkout", "stage-branch"])
            .output()
            .unwrap();

        // Add a new file and stage it
        let test_file = git_temp.path().join("staged.txt");
        std::fs::write(&test_file, "Staged content\nAnother line\n").unwrap();
        git_service.stage_all_changes().unwrap();

        let stats = calculate_diff_stats(git_temp.path(), "main").unwrap();
        // Staged changes are included (since we're on the same commit as main,
        // only staged/unstaged changes will show up)
        assert_eq!(stats.additions, 2);
        assert_eq!(stats.deletions, 0);
    }

    #[test]
    fn test_find_parent_branch_with_upstream() {
        let (git_temp, git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Create and checkout develop branch
        git_service.create_branch("develop", "main").unwrap();
        std::process::Command::new("git")
            .current_dir(git_temp.path())
            .args(["checkout", "develop"])
            .output()
            .unwrap();

        // Create feature branch from develop
        git_service
            .create_branch("feature/test", "develop")
            .unwrap();
        std::process::Command::new("git")
            .current_dir(git_temp.path())
            .args(["checkout", "feature/test"])
            .output()
            .unwrap();

        let parent = find_parent_branch(git_temp.path(), "feature/test").unwrap();
        // Should find develop as it's a common branch and has merge-base
        assert!(parent == "develop" || parent == "main");
    }

    #[test]
    fn test_find_parent_branch_main_as_current() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // When current branch is main, it should return main
        let parent = find_parent_branch(git_temp.path(), "main").unwrap();
        assert_eq!(parent, "main");
    }

    #[test]
    fn test_calculate_diff_stats_path_not_exists() {
        let non_existent = Path::new("/non/existent/path");
        let result = calculate_diff_stats(non_existent, "main");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_diff_stats_equality() {
        let stats1 = DiffStats::new(10, 5);
        let stats2 = DiffStats::new(10, 5);
        let stats3 = DiffStats::new(10, 6);

        assert_eq!(stats1, stats2);
        assert_ne!(stats1, stats3);
    }
}
