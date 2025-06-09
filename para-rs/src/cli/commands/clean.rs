use crate::cli::parser::CleanArgs;
use crate::core::git::{GitOperations, GitService};
use crate::utils::{ParaError, Result};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn execute(args: CleanArgs) -> Result<()> {
    let git_service = GitService::discover()?;
    let mut cleaner = SessionCleaner::new(git_service, args.backups);
    cleaner.execute_clean()
}

struct SessionCleaner {
    git_service: GitService,
    clean_backups: bool,
    sessions_discovered: Vec<SessionInfo>,
    archived_branches: Vec<String>,
    current_directory: Option<PathBuf>,
}

#[derive(Debug)]
struct SessionInfo {
    session_id: String,
    branch_name: String,
    worktree_path: PathBuf,
    state_files: Vec<PathBuf>,
    exists_as_worktree: bool,
    exists_as_branch: bool,
}

impl SessionCleaner {
    fn new(git_service: GitService, clean_backups: bool) -> Self {
        let current_directory = std::env::current_dir().ok();
        Self {
            git_service,
            clean_backups,
            sessions_discovered: Vec::new(),
            archived_branches: Vec::new(),
            current_directory,
        }
    }

    fn execute_clean(&mut self) -> Result<()> {
        self.discover_sessions()?;
        self.discover_archived_branches()?;

        if self.sessions_discovered.is_empty() && self.archived_branches.is_empty() {
            println!("No active sessions or archived branches found to clean.");
            return Ok(());
        }

        // Check for current directory safety
        self.check_current_directory_safety()?;

        self.show_cleanup_summary();

        if !self.confirm_cleanup()? {
            println!("Clean operation cancelled.");
            return Ok(());
        }

        self.perform_cleanup()
    }

    fn discover_sessions(&mut self) -> Result<()> {
        let state_dir = self.git_service.repository().root.join(".para_state");
        if !state_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(&state_dir).map_err(|e| {
            ParaError::file_operation(format!(
                "Failed to read state directory {}: {}",
                state_dir.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                ParaError::file_operation(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".state") {
                    if let Some(session_id) = file_name.strip_suffix(".state") {
                        if let Ok(session_info) = self.parse_session_info(session_id, &state_dir) {
                            self.sessions_discovered.push(session_info);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn check_current_directory_safety(&self) -> Result<()> {
        if let Some(ref current_dir) = self.current_directory {
            for session in &self.sessions_discovered {
                if current_dir.starts_with(&session.worktree_path) {
                    println!(
                        "⚠️  Warning: You are currently inside session '{}' worktree.",
                        session.session_id
                    );
                    println!("   Current directory: {}", current_dir.display());
                    println!("   Session worktree: {}", session.worktree_path.display());
                    println!();
                    println!(
                        "💡 Cleaning this session will remove the directory you're currently in."
                    );
                    println!("   Please navigate to a different directory before cleaning.");
                    return Err(ParaError::invalid_args(
                        "Cannot clean session while inside its worktree directory",
                    ));
                }
            }
        }
        Ok(())
    }

    fn parse_session_info(&self, session_id: &str, state_dir: &Path) -> Result<SessionInfo> {
        let state_file = state_dir.join(format!("{}.state", session_id));
        let content = fs::read_to_string(&state_file)
            .map_err(|e| ParaError::file_operation(format!("Failed to read state file: {}", e)))?;

        let parts: Vec<&str> = content.trim().split('|').collect();
        if parts.len() < 3 {
            return Err(ParaError::config_error(format!(
                "Invalid state file format for session {}",
                session_id
            )));
        }

        let branch_name = parts[0].to_string();
        let worktree_path = PathBuf::from(parts[1]);

        let mut state_files = vec![state_file];
        for suffix in &[".prompt", ".launch"] {
            let file_path = state_dir.join(format!("{}{}", session_id, suffix));
            if file_path.exists() {
                state_files.push(file_path);
            }
        }

        let exists_as_branch = self.git_service.branch_exists(&branch_name)?;
        let exists_as_worktree = worktree_path.exists()
            && self
                .git_service
                .worktree_manager()
                .validate_worktree(&worktree_path)
                .is_ok();

        Ok(SessionInfo {
            session_id: session_id.to_string(),
            branch_name,
            worktree_path,
            state_files,
            exists_as_worktree,
            exists_as_branch,
        })
    }

    fn discover_archived_branches(&mut self) -> Result<()> {
        if !self.clean_backups {
            return Ok(());
        }

        let branch_manager = self.git_service.branch_manager();

        for prefix in &["para", "pc"] {
            let archived = branch_manager.list_archived_branches(prefix)?;
            self.archived_branches.extend(archived);
        }

        Ok(())
    }

    fn show_cleanup_summary(&self) {
        println!("Clean operation will remove:");
        println!();

        if !self.sessions_discovered.is_empty() {
            println!("Active sessions ({}):", self.sessions_discovered.len());
            for session in &self.sessions_discovered {
                println!("  • {} ({})", session.session_id, session.branch_name);
                if session.exists_as_worktree {
                    println!("    - Worktree: {}", session.worktree_path.display());
                }
                if session.exists_as_branch {
                    println!("    - Branch: {}", session.branch_name);
                }
                println!("    - State files: {} files", session.state_files.len());
            }
            println!();
        }

        if !self.archived_branches.is_empty() {
            println!("Archived branches ({}):", self.archived_branches.len());
            for branch in &self.archived_branches {
                println!("  • {}", branch);
            }
            println!();
        }

        if self.sessions_discovered.is_empty() && self.archived_branches.is_empty() {
            println!("No items to clean.");
        }
    }

    fn confirm_cleanup(&self) -> Result<bool> {
        if self.sessions_discovered.is_empty() && self.archived_branches.is_empty() {
            return Ok(false);
        }

        print!("This action cannot be undone. Continue? [y/N]: ");
        io::stdout()
            .flush()
            .map_err(|e| ParaError::file_operation(format!("Failed to flush stdout: {}", e)))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| ParaError::file_operation(format!("Failed to read input: {}", e)))?;

        let response = input.trim().to_lowercase();
        Ok(response == "y" || response == "yes")
    }

    fn perform_cleanup(&self) -> Result<()> {
        let mut cleanup_summary = CleanupSummary::new();

        for session in &self.sessions_discovered {
            if let Err(e) = self.clean_session(session, &mut cleanup_summary) {
                eprintln!("Failed to clean session {}: {}", session.session_id, e);
                cleanup_summary
                    .errors
                    .push(format!("Session {}: {}", session.session_id, e));
            }
        }

        for archived_branch in &self.archived_branches {
            if let Err(e) = self.clean_archived_branch(archived_branch, &mut cleanup_summary) {
                eprintln!("Failed to clean archived branch {}: {}", archived_branch, e);
                cleanup_summary
                    .errors
                    .push(format!("Branch {}: {}", archived_branch, e));
            }
        }

        self.cleanup_empty_directories(&mut cleanup_summary)?;
        self.show_cleanup_results(&cleanup_summary);

        Ok(())
    }

    fn clean_session(&self, session: &SessionInfo, summary: &mut CleanupSummary) -> Result<()> {
        if session.exists_as_worktree {
            if let Err(e) = self.git_service.remove_worktree(&session.worktree_path) {
                eprintln!(
                    "Warning: Failed to remove worktree {}: {}",
                    session.worktree_path.display(),
                    e
                );
                if let Err(force_err) = self
                    .git_service
                    .worktree_manager()
                    .force_remove_worktree(&session.worktree_path)
                {
                    return Err(ParaError::git_operation(format!(
                        "Failed to force remove worktree {}: {}",
                        session.worktree_path.display(),
                        force_err
                    )));
                }
            }
            summary.worktrees_removed += 1;
        }

        if session.exists_as_branch {
            self.git_service.delete_branch(&session.branch_name, true)?;
            summary.branches_removed += 1;
        }

        for state_file in &session.state_files {
            if state_file.exists() {
                fs::remove_file(state_file).map_err(|e| {
                    ParaError::file_operation(format!(
                        "Failed to remove state file {}: {}",
                        state_file.display(),
                        e
                    ))
                })?;
                summary.state_files_removed += 1;
            }
        }

        summary.sessions_cleaned += 1;
        Ok(())
    }

    fn clean_archived_branch(&self, branch_name: &str, summary: &mut CleanupSummary) -> Result<()> {
        self.git_service.delete_branch(branch_name, true)?;
        summary.archived_branches_removed += 1;
        Ok(())
    }

    fn cleanup_empty_directories(&self, summary: &mut CleanupSummary) -> Result<()> {
        let state_dir = self.git_service.repository().root.join(".para_state");
        if state_dir.exists() {
            if let Ok(entries) = fs::read_dir(&state_dir) {
                if entries.count() == 0 && fs::remove_dir(&state_dir).is_ok() {
                    summary.directories_removed += 1;
                }
            }
        }

        let repo_root = self.git_service.repository().root.clone();
        for prefix in &["subtrees/para", "subtrees/pc"] {
            let subtrees_dir = repo_root.join(prefix);
            if subtrees_dir.exists() {
                if let Ok(entries) = fs::read_dir(&subtrees_dir) {
                    if entries.count() == 0 {
                        let _ = fs::remove_dir(&subtrees_dir);
                    }
                }
            }
        }

        Ok(())
    }

    fn show_cleanup_results(&self, summary: &CleanupSummary) {
        println!();
        println!("Cleanup completed:");
        println!("  Sessions cleaned: {}", summary.sessions_cleaned);
        println!("  Worktrees removed: {}", summary.worktrees_removed);
        println!("  Branches removed: {}", summary.branches_removed);
        println!("  State files removed: {}", summary.state_files_removed);

        if self.clean_backups {
            println!(
                "  Archived branches removed: {}",
                summary.archived_branches_removed
            );
        }

        if summary.directories_removed > 0 {
            println!(
                "  Empty directories removed: {}",
                summary.directories_removed
            );
        }

        if !summary.errors.is_empty() {
            println!();
            println!("Errors encountered:");
            for error in &summary.errors {
                println!("  • {}", error);
            }
        }
    }
}

#[derive(Debug)]
struct CleanupSummary {
    sessions_cleaned: usize,
    worktrees_removed: usize,
    branches_removed: usize,
    state_files_removed: usize,
    archived_branches_removed: usize,
    directories_removed: usize,
    errors: Vec<String>,
}

impl CleanupSummary {
    fn new() -> Self {
        Self {
            sessions_cleaned: 0,
            worktrees_removed: 0,
            branches_removed: 0,
            state_files_removed: 0,
            archived_branches_removed: 0,
            directories_removed: 0,
            errors: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, GitService) {
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

        let service = GitService::discover_from(repo_path).expect("Failed to discover repo");
        (temp_dir, service)
    }

    fn create_mock_session_state(
        temp_dir: &TempDir,
        session_id: &str,
        branch_name: &str,
        worktree_path: &str,
    ) -> PathBuf {
        let state_dir = temp_dir.path().join(".para_state");
        fs::create_dir_all(&state_dir).expect("Failed to create state dir");

        let state_file = state_dir.join(format!("{}.state", session_id));
        let content = format!("{}|{}|master|squash", branch_name, worktree_path);
        fs::write(&state_file, content).expect("Failed to write state file");

        let prompt_file = state_dir.join(format!("{}.prompt", session_id));
        fs::write(&prompt_file, "test prompt").expect("Failed to write prompt file");

        state_file
    }

    #[test]
    fn test_execute_clean_no_sessions() {
        let (_temp_dir, service) = setup_test_repo();
        let mut cleaner = SessionCleaner::new(service, false);

        let result = cleaner.execute_clean();
        assert!(result.is_ok());
    }

    #[test]
    fn test_session_info_parsing() {
        let (temp_dir, service) = setup_test_repo();
        let cleaner = SessionCleaner::new(service, false);

        let session_id = "test-session";
        let branch_name = "para/test-session";
        let worktree_path = temp_dir.path().join("subtrees").join("test-session");

        create_mock_session_state(
            &temp_dir,
            session_id,
            branch_name,
            &worktree_path.to_string_lossy(),
        );

        let state_dir = temp_dir.path().join(".para_state");
        let result = cleaner.parse_session_info(session_id, &state_dir);

        assert!(result.is_ok());
        let session_info = result.unwrap();
        assert_eq!(session_info.session_id, session_id);
        assert_eq!(session_info.branch_name, branch_name);
        assert_eq!(session_info.worktree_path, worktree_path);
        assert_eq!(session_info.state_files.len(), 2); // .state and .prompt files
    }

    #[test]
    fn test_session_info_parsing_invalid_format() {
        let (temp_dir, service) = setup_test_repo();
        let cleaner = SessionCleaner::new(service, false);

        let state_dir = temp_dir.path().join(".para_state");
        fs::create_dir_all(&state_dir).expect("Failed to create state dir");

        let state_file = state_dir.join("invalid.state");
        fs::write(&state_file, "invalid|format").expect("Failed to write invalid state file");

        let result = cleaner.parse_session_info("invalid", &state_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_discover_sessions() {
        let (temp_dir, service) = setup_test_repo();
        let mut cleaner = SessionCleaner::new(service, false);

        create_mock_session_state(
            &temp_dir,
            "session1",
            "para/session1",
            &temp_dir
                .path()
                .join("subtrees")
                .join("session1")
                .to_string_lossy(),
        );
        create_mock_session_state(
            &temp_dir,
            "session2",
            "para/session2",
            &temp_dir
                .path()
                .join("subtrees")
                .join("session2")
                .to_string_lossy(),
        );

        // Test discover_sessions by modifying the cleaner to use the test repo path
        // Instead of changing current directory, we'll work within the test directory
        let result = cleaner.discover_sessions();

        assert!(result.is_ok());
        assert_eq!(cleaner.sessions_discovered.len(), 2);
    }

    #[test]
    fn test_discover_sessions_empty_state_dir() {
        let (_temp_dir, service) = setup_test_repo();
        let mut cleaner = SessionCleaner::new(service, false);

        // Test discover_sessions by working within the test directory
        // Instead of changing current directory, we'll work within the test directory
        let result = cleaner.discover_sessions();
        assert!(result.is_ok());
        assert!(cleaner.sessions_discovered.is_empty());
    }

    #[test]
    fn test_discover_archived_branches() {
        let (_temp_dir, service) = setup_test_repo();

        let branch_manager = service.branch_manager();
        let base_branch = service
            .get_current_branch()
            .expect("Failed to get current branch");

        service
            .create_branch("test-branch", &base_branch)
            .expect("Failed to create branch");

        branch_manager
            .switch_to_branch(&base_branch)
            .expect("Failed to checkout base branch");

        let _archived_name = branch_manager
            .move_to_archive("test-branch", "para")
            .expect("Failed to archive branch");

        let mut cleaner = SessionCleaner::new(service, true);
        let result = cleaner.discover_archived_branches();
        assert!(result.is_ok());
    }

    #[test]
    fn test_discover_archived_branches_disabled() {
        let (_temp_dir, service) = setup_test_repo();
        let mut cleaner = SessionCleaner::new(service, false);

        let result = cleaner.discover_archived_branches();
        assert!(result.is_ok());
        assert!(cleaner.archived_branches.is_empty());
    }

    #[test]
    fn test_cleanup_summary_new() {
        let summary = CleanupSummary::new();
        assert_eq!(summary.sessions_cleaned, 0);
        assert_eq!(summary.worktrees_removed, 0);
        assert_eq!(summary.branches_removed, 0);
        assert_eq!(summary.state_files_removed, 0);
        assert_eq!(summary.archived_branches_removed, 0);
        assert_eq!(summary.directories_removed, 0);
        assert!(summary.errors.is_empty());
    }

    #[test]
    fn test_clean_args_backups_flag() {
        let args = CleanArgs { backups: true };
        assert!(args.backups);

        let args = CleanArgs { backups: false };
        assert!(!args.backups);
    }

    #[test]
    fn test_current_directory_safety_check() {
        let (temp_dir, service) = setup_test_repo();
        let mut cleaner = SessionCleaner::new(service, false);

        let session_id = "test-session";
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path).expect("Failed to create worktree dir");

        create_mock_session_state(
            &temp_dir,
            session_id,
            "para/test-session",
            &worktree_path.to_string_lossy(),
        );

        // Create a session info that represents being inside the worktree
        let session_info = SessionInfo {
            session_id: session_id.to_string(),
            branch_name: "para/test-session".to_string(),
            worktree_path: worktree_path.clone(),
            state_files: vec![],
            exists_as_worktree: true,
            exists_as_branch: false,
        };

        cleaner.sessions_discovered.push(session_info);

        // Simulate being inside the worktree directory
        cleaner.current_directory = Some(worktree_path.join("subdirectory"));

        let result = cleaner.check_current_directory_safety();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot clean session while inside its worktree directory"));
    }

    #[test]
    fn test_current_directory_safety_check_outside_worktree() {
        let (temp_dir, service) = setup_test_repo();
        let mut cleaner = SessionCleaner::new(service, false);

        let session_id = "test-session";
        let worktree_path = temp_dir.path().join("test-worktree");

        let session_info = SessionInfo {
            session_id: session_id.to_string(),
            branch_name: "para/test-session".to_string(),
            worktree_path: worktree_path.clone(),
            state_files: vec![],
            exists_as_worktree: true,
            exists_as_branch: false,
        };

        cleaner.sessions_discovered.push(session_info);

        // Simulate being outside the worktree directory
        cleaner.current_directory = Some(temp_dir.path().join("different-directory"));

        let result = cleaner.check_current_directory_safety();
        assert!(result.is_ok());
    }
}
