use chrono::{DateTime, Utc};
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

/// Detect the last activity time for a session worktree
///
/// This approach is used by popular git repos like GitLens and Tower.
/// (with the exception that they also user FSMonitor for large repositories, we don't it's to complex)
/// Uses a three-tier strategy for optimal performance:
/// 1. Lightweight file monitoring of git internals (fastest)
/// 2. Git plumbing commands for actual change detection
/// 3. Fallback to last commit time
pub fn detect_last_activity(worktree_path: &Path) -> Option<DateTime<Utc>> {
    let mut latest_time: Option<DateTime<Utc>> = None;

    // Tier 1: Quick check of git internal files for activity hints
    if let Some(git_time) = check_git_internal_files(worktree_path) {
        latest_time = Some(git_time);
    }

    // Tier 2: Check for actual changes using efficient git plumbing commands
    // This should always run to get the most accurate time
    if let Some(change_time) = get_latest_change_time_efficient(worktree_path) {
        match latest_time {
            None => latest_time = Some(change_time),
            Some(current) if change_time > current => latest_time = Some(change_time),
            _ => {}
        }
    }

    // If we have any activity time, return it
    if latest_time.is_some() {
        return latest_time;
    }

    // Tier 3: No changes, fall back to last commit time
    get_last_commit_time(worktree_path)
}

/// Tier 1: Check git internal files for quick activity detection
fn check_git_internal_files(worktree_path: &Path) -> Option<DateTime<Utc>> {
    let git_dir = find_git_dir(worktree_path);
    let mut latest_time: Option<SystemTime> = None;

    // Check critical activity indicator files
    let activity_files = [
        git_dir.join("logs/HEAD"),      // Ref changes (commits, checkouts)
        git_dir.join("index"),          // Staging area changes
        git_dir.join("FETCH_HEAD"),     // Recent fetches
        git_dir.join("refs/stash"),     // Stash operations
        git_dir.join("COMMIT_EDITMSG"), // Recent commit message
    ];

    for file in &activity_files {
        if let Ok(metadata) = std::fs::metadata(file) {
            if let Ok(modified) = metadata.modified() {
                match latest_time {
                    None => latest_time = Some(modified),
                    Some(current) if modified > current => latest_time = Some(modified),
                    _ => {}
                }
            }
        }
    }

    latest_time.and_then(system_time_to_datetime)
}

/// Find the actual git directory, handling worktrees
fn find_git_dir(worktree_path: &Path) -> std::path::PathBuf {
    let dot_git = worktree_path.join(".git");

    // Check if .git is a file (worktree) or directory (regular repo)
    if dot_git.is_file() {
        // It's a worktree, read the gitdir path
        if let Ok(contents) = std::fs::read_to_string(&dot_git) {
            if let Some(gitdir_line) = contents.lines().find(|line| line.starts_with("gitdir:")) {
                let gitdir = gitdir_line.trim_start_matches("gitdir:").trim();
                return Path::new(gitdir).to_path_buf();
            }
        }
    }

    // Regular repository or fallback
    dot_git
}

/// Tier 2: Get the most recent modification time from changed files using efficient git commands
fn get_latest_change_time_efficient(worktree_path: &Path) -> Option<DateTime<Utc>> {
    let mut latest_time: Option<SystemTime> = None;

    // Check unstaged changes
    if let Some(time) = get_unstaged_changes_time(worktree_path) {
        latest_time = Some(time);
    }

    // Check staged changes (might be older than unstaged)
    if let Some(time) = get_staged_changes_time(worktree_path) {
        match latest_time {
            None => latest_time = Some(time),
            Some(current) if time > current => latest_time = Some(time),
            _ => {}
        }
    }

    // Check untracked files
    if let Some(time) = get_untracked_files_time(worktree_path) {
        match latest_time {
            None => latest_time = Some(time),
            Some(current) if time > current => latest_time = Some(time),
            _ => {}
        }
    }

    latest_time.and_then(system_time_to_datetime)
}

/// Get modification time of unstaged changes
fn get_unstaged_changes_time(worktree_path: &Path) -> Option<SystemTime> {
    let output = Command::new("git")
        .args(["diff-files", "--name-only"])
        .current_dir(worktree_path)
        .output()
        .ok()?;

    if !output.status.success() || output.stdout.is_empty() {
        return None;
    }

    let mut latest: Option<SystemTime> = None;
    for line in output
        .stdout
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
    {
        if let Ok(filename) = std::str::from_utf8(line) {
            let filepath = worktree_path.join(filename);
            if let Ok(metadata) = std::fs::metadata(&filepath) {
                if let Ok(modified) = metadata.modified() {
                    match latest {
                        None => latest = Some(modified),
                        Some(current) if modified > current => latest = Some(modified),
                        _ => {}
                    }
                }
            }
        }
    }

    latest
}

/// Get modification time of staged changes
fn get_staged_changes_time(worktree_path: &Path) -> Option<SystemTime> {
    // For staged changes, the best indicator is the index modification time
    let git_dir = find_git_dir(worktree_path);
    let index_path = git_dir.join("index");

    // First check if there are actually staged changes
    let output = Command::new("git")
        .args(["diff-index", "--quiet", "--cached", "HEAD"])
        .current_dir(worktree_path)
        .status();

    if let Ok(status) = output {
        if status.success() {
            return None; // No staged changes
        }
    }

    // Return index modification time
    std::fs::metadata(&index_path)
        .ok()
        .and_then(|m| m.modified().ok())
}

/// Get modification time of untracked files
fn get_untracked_files_time(worktree_path: &Path) -> Option<SystemTime> {
    let output = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(worktree_path)
        .output()
        .ok()?;

    if !output.status.success() || output.stdout.is_empty() {
        return None;
    }

    let mut latest: Option<SystemTime> = None;
    for line in output
        .stdout
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
    {
        if let Ok(filename) = std::str::from_utf8(line) {
            let filepath = worktree_path.join(filename);
            if let Ok(metadata) = std::fs::metadata(&filepath) {
                if let Ok(modified) = metadata.modified() {
                    match latest {
                        None => latest = Some(modified),
                        Some(current) if modified > current => latest = Some(modified),
                        _ => {}
                    }
                }
            }
        }
    }

    latest
}

/// Get the last commit time
fn get_last_commit_time(worktree_path: &Path) -> Option<DateTime<Utc>> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%ct"])
        .current_dir(worktree_path)
        .output()
        .ok()?;

    if output.status.success() {
        let timestamp_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        timestamp_str
            .parse::<i64>()
            .ok()
            .and_then(|ts| DateTime::from_timestamp(ts, 0))
    } else {
        None
    }
}

/// Convert SystemTime to DateTime<Utc>
fn system_time_to_datetime(time: SystemTime) -> Option<DateTime<Utc>> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .and_then(|duration| DateTime::from_timestamp(duration.as_secs() as i64, 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::git::GitOperations;
    use crate::core::git::GitService;
    use std::fs;
    use std::thread;
    use std::time::Duration as StdDuration;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, GitService) {
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

        let service = GitService::discover_from(repo_path).expect("Failed to discover repo");
        (temp_dir, service)
    }

    #[test]
    fn test_system_time_conversion() {
        let now = SystemTime::now();
        let datetime = system_time_to_datetime(now);
        assert!(datetime.is_some());

        // Should be very close to current time
        if let Some(dt) = datetime {
            let now_utc = Utc::now();
            let diff = (now_utc - dt).num_seconds().abs();
            assert!(diff < 2);
        }
    }

    #[test]
    fn test_activity_detection_no_changes() {
        let (git_temp, _git_service) = setup_test_repo();

        // No changes, should return last commit time
        let activity = detect_last_activity(git_temp.path());
        assert!(activity.is_some());

        // Activity should be very recent (just created the repo)
        if let Some(time) = activity {
            let now = Utc::now();
            let diff = now - time;
            assert!(
                diff.num_seconds() < 5,
                "Expected recent commit, but was {} seconds ago",
                diff.num_seconds()
            );
        }
    }

    #[test]
    fn test_activity_detection_with_unstaged_changes() {
        let (git_temp, _git_service) = setup_test_repo();

        // Make a change to an existing file
        let readme_path = git_temp.path().join("README.md");
        thread::sleep(StdDuration::from_millis(1100)); // Ensure different timestamp
        fs::write(&readme_path, "# Test Repository\n\nModified content").unwrap();

        // Should detect the modification time of the changed file
        let activity = detect_last_activity(git_temp.path()).unwrap();
        let now = Utc::now();
        let diff = now - activity;

        // Should be very recent (within 2 seconds)
        assert!(
            diff.num_seconds() < 3,
            "Expected recent activity from modified file, but was {} seconds ago",
            diff.num_seconds()
        );
    }

    #[test]
    fn test_activity_detection_with_new_file() {
        let (git_temp, _git_service) = setup_test_repo();

        // Create a new untracked file
        thread::sleep(StdDuration::from_millis(1100)); // Ensure different timestamp
        let new_file = git_temp.path().join("new_file.txt");
        fs::write(&new_file, "New content").unwrap();

        // Should detect the new file's modification time
        let activity = detect_last_activity(git_temp.path()).unwrap();
        let now = Utc::now();
        let diff = now - activity;

        assert!(
            diff.num_seconds() < 3,
            "Expected recent activity from new file, but was {} seconds ago",
            diff.num_seconds()
        );
    }

    #[test]
    fn test_activity_detection_with_staged_changes() {
        let (git_temp, _git_service) = setup_test_repo();

        // Create and stage a new file
        thread::sleep(StdDuration::from_millis(1100)); // Ensure different timestamp
        let new_file = git_temp.path().join("staged.txt");
        fs::write(&new_file, "Staged content").unwrap();

        Command::new("git")
            .current_dir(git_temp.path())
            .args(["add", "staged.txt"])
            .status()
            .expect("Failed to stage file");

        // Should still detect the file's modification time
        let activity = detect_last_activity(git_temp.path()).unwrap();
        let now = Utc::now();
        let diff = now - activity;

        assert!(
            diff.num_seconds() < 3,
            "Expected recent activity from staged file, but was {} seconds ago",
            diff.num_seconds()
        );
    }

    #[test]
    fn test_activity_detection_multiple_changes() {
        let (git_temp, _git_service) = setup_test_repo();

        // Create multiple files with different timestamps
        let file1 = git_temp.path().join("file1.txt");
        fs::write(&file1, "Content 1").unwrap();

        thread::sleep(StdDuration::from_millis(1100));

        let file2 = git_temp.path().join("file2.txt");
        fs::write(&file2, "Content 2").unwrap();

        thread::sleep(StdDuration::from_millis(1100));

        // Modify the README (newest change)
        let readme = git_temp.path().join("README.md");
        fs::write(&readme, "# Modified README").unwrap();

        // Should return the most recent modification time (README)
        let activity = detect_last_activity(git_temp.path()).unwrap();
        let now = Utc::now();
        let diff = now - activity;

        assert!(
            diff.num_seconds() < 5,
            "Expected activity from most recent file change, but was {} seconds ago",
            diff.num_seconds()
        );
    }

    #[test]
    fn test_activity_detection_ignored_files() {
        let (git_temp, _git_service) = setup_test_repo();

        // Create a .gitignore file
        fs::write(git_temp.path().join(".gitignore"), "*.log\ntemp/\n").unwrap();

        // Commit the .gitignore
        Command::new("git")
            .current_dir(git_temp.path())
            .args(["add", ".gitignore"])
            .status()
            .expect("Failed to add .gitignore");

        Command::new("git")
            .current_dir(git_temp.path())
            .args(["commit", "-m", "Add .gitignore"])
            .status()
            .expect("Failed to commit .gitignore");

        let initial_activity = detect_last_activity(git_temp.path()).unwrap();

        // Create ignored files (should not affect activity)
        thread::sleep(StdDuration::from_millis(1100));
        fs::write(git_temp.path().join("test.log"), "Log content").unwrap();
        fs::create_dir(git_temp.path().join("temp")).unwrap();
        fs::write(git_temp.path().join("temp/file.txt"), "Temp content").unwrap();

        // Activity should still be the last commit (ignored files don't count)
        let activity_after_ignored = detect_last_activity(git_temp.path()).unwrap();

        // Should be the same as initial (within 1 second tolerance for test timing)
        let diff = (activity_after_ignored - initial_activity)
            .num_seconds()
            .abs();
        assert!(diff <= 1, "Ignored files should not affect activity time");
    }

    #[test]
    fn test_activity_detection_worktree() {
        let (git_temp, git_service) = setup_test_repo();

        // Create a worktree
        let worktree_path = git_temp.path().join("test-worktree");
        git_service
            .create_worktree("test-branch", &worktree_path)
            .expect("Failed to create worktree");

        // Make a change in the worktree
        thread::sleep(StdDuration::from_millis(1100));
        let worktree_file = worktree_path.join("worktree_file.txt");
        fs::write(&worktree_file, "Worktree content").unwrap();

        // Activity detection should work in the worktree
        let activity = detect_last_activity(&worktree_path).unwrap();
        let now = Utc::now();
        let diff = now - activity;

        assert!(
            diff.num_seconds() < 3,
            "Expected recent activity in worktree, but was {} seconds ago",
            diff.num_seconds()
        );
    }

    #[test]
    fn test_get_latest_change_time_handles_deleted_files() {
        let (git_temp, _git_service) = setup_test_repo();

        // Create and commit a file
        let test_file = git_temp.path().join("to_delete.txt");
        fs::write(&test_file, "Will be deleted").unwrap();

        Command::new("git")
            .current_dir(git_temp.path())
            .args(["add", "to_delete.txt"])
            .status()
            .expect("Failed to add file");

        Command::new("git")
            .current_dir(git_temp.path())
            .args(["commit", "-m", "Add file to delete"])
            .status()
            .expect("Failed to commit");

        // Delete the file
        fs::remove_file(&test_file).unwrap();

        // Should handle the deleted file gracefully and fall back to commit time
        let activity = detect_last_activity(git_temp.path());
        assert!(activity.is_some(), "Should handle deleted files gracefully");
    }

    #[test]
    fn test_git_internal_files_detection() {
        let (git_temp, _git_service) = setup_test_repo();

        // Check that we can detect git internal files
        let git_files_time = check_git_internal_files(git_temp.path());
        assert!(git_files_time.is_some(), "Should detect git internal files");

        // The time should be recent (from initial commit)
        if let Some(time) = git_files_time {
            let now = Utc::now();
            let diff = now - time;
            assert!(
                diff.num_seconds() < 5,
                "Git files should show recent activity"
            );
        }
    }
}
