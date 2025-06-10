use crate::cli::parser::ListArgs;
use crate::config::ConfigManager;
use crate::core::git::{BranchInfo, GitOperations, GitService, WorktreeInfo};
use crate::core::session::{SessionManager, SessionStatus as UnifiedSessionStatus};
use crate::utils::{ParaError, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub base_branch: String,
    pub merge_mode: String,
    pub status: SessionStatus,
    pub last_modified: Option<DateTime<Utc>>,
    pub commit_count: Option<usize>,
    pub has_uncommitted_changes: Option<bool>,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Active,
    Dirty,
    Missing,
    Archived,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Active => "active",
            SessionStatus::Dirty => "dirty",
            SessionStatus::Missing => "missing",
            SessionStatus::Archived => "archived",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            SessionStatus::Active => "✓",
            SessionStatus::Dirty => "●",
            SessionStatus::Missing => "✗",
            SessionStatus::Archived => "📦",
        }
    }
}

pub fn execute(args: ListArgs) -> Result<()> {
    let config = ConfigManager::load_or_create()
        .map_err(|e| ParaError::config_error(format!("Failed to load configuration: {}", e)))?;

    let session_manager = SessionManager::new(config)?;

    let sessions = if args.archived {
        list_archived_sessions(&session_manager)?
    } else {
        list_active_sessions(&session_manager)?
    };

    if sessions.is_empty() {
        if args.archived {
            println!("No archived sessions found.");
        } else {
            println!("No active sessions found.");
        }
        return Ok(());
    }

    display_sessions(&sessions, &args)?;
    Ok(())
}

fn list_active_sessions(session_manager: &SessionManager) -> Result<Vec<SessionInfo>> {
    let session_states = session_manager.list_sessions()?;
    let git_service = GitService::discover()?;
    
    let mut sessions = Vec::new();
    
    for session_state in session_states {
        let has_uncommitted_changes = if session_state.worktree_path.exists() {
            git_service_for_path(&session_state.worktree_path)
                .and_then(|service| service.has_uncommitted_changes().ok())
        } else {
            None
        };

        let is_current = std::env::current_dir()
            .map(|cwd| cwd.starts_with(&session_state.worktree_path))
            .unwrap_or(false);
        
        let status = determine_unified_session_status(&session_state, &git_service)?;

        let session_info = SessionInfo {
            session_id: session_state.name.clone(),
            branch: session_state.branch.clone(),
            worktree_path: session_state.worktree_path.clone(),
            base_branch: "main".to_string(), // Simplified for now
            merge_mode: "squash".to_string(), // Default for now
            status,
            last_modified: Some(session_state.created_at),
            commit_count: Some(0), // Simplified for now
            has_uncommitted_changes,
            is_current,
        };
        
        sessions.push(session_info);
    }

    sessions.sort_by(|a, b| {
        b.last_modified
            .unwrap_or(DateTime::<Utc>::MIN_UTC)
            .cmp(&a.last_modified.unwrap_or(DateTime::<Utc>::MIN_UTC))
    });

    Ok(sessions)
}

fn list_archived_sessions(session_manager: &SessionManager) -> Result<Vec<SessionInfo>> {
    let git_service = GitService::discover()?;
    let branch_manager = git_service.branch_manager();
    let archived_branches = branch_manager.list_archived_branches("para")?;

    let mut sessions = Vec::new();

    for branch_name in archived_branches {
        if let Some(session_id) = extract_session_id_from_archived_branch(&branch_name) {
            let session_info = SessionInfo {
                session_id: session_id.clone(),
                branch: branch_name.clone(),
                worktree_path: PathBuf::new(),
                base_branch: "unknown".to_string(),
                merge_mode: "unknown".to_string(),
                status: SessionStatus::Archived,
                last_modified: None,
                commit_count: None,
                has_uncommitted_changes: None,
                is_current: false,
            };
            sessions.push(session_info);
        }
    }

    Ok(sessions)
}

fn determine_unified_session_status(
    session_state: &crate::core::session::SessionState,
    git_service: &GitService,
) -> Result<SessionStatus> {
    // Check if worktree path exists
    if !session_state.worktree_path.exists() {
        return Ok(SessionStatus::Missing);
    }

    // Check session status first
    match session_state.status {
        UnifiedSessionStatus::Cancelled | UnifiedSessionStatus::Finished => {
            return Ok(SessionStatus::Archived);
        }
        _ => {}
    }

    // Check if worktree is registered with git
    let worktrees = git_service.list_worktrees()?;
    let worktree_exists = worktrees.iter()
        .any(|w| w.path == session_state.worktree_path);
    
    if !worktree_exists {
        return Ok(SessionStatus::Missing);
    }

    // Check for uncommitted changes
    if let Some(service) = git_service_for_path(&session_state.worktree_path) {
        if let Ok(is_clean) = service.is_clean_working_tree() {
            if !is_clean {
                return Ok(SessionStatus::Dirty);
            }
        }
    }

    Ok(SessionStatus::Active)
}

// Removed old determine_session_status - using unified session system

fn git_service_for_path(path: &Path) -> Option<GitService> {
    GitService::discover_from(path).ok()
}

// Removed get_last_modified_time - using unified session system metadata

fn extract_session_id_from_archived_branch(branch_name: &str) -> Option<String> {
    if let Some(stripped) = branch_name.strip_prefix("para/archived/") {
        if let Some(session_part) = stripped.split('/').next_back() {
            return Some(session_part.to_string());
        }
    }
    None
}

fn display_sessions(sessions: &[SessionInfo], args: &ListArgs) -> Result<()> {
    if args.verbose {
        display_verbose_sessions(sessions)
    } else {
        display_compact_sessions(sessions)
    }
}

fn display_compact_sessions(sessions: &[SessionInfo]) -> Result<()> {
    println!(
        "{:<2} {:<30} {:<20} {:<15}",
        "St", "Session", "Branch", "Status"
    );
    println!("{}", "-".repeat(70));

    for session in sessions {
        let current_marker = if session.is_current { "*" } else { " " };
        let status_indicator = session.status.symbol();

        println!(
            "{}{} {:<30} {:<20} {:<15}",
            current_marker,
            status_indicator,
            truncate_string(&session.session_id, 30),
            truncate_string(&session.branch, 20),
            session.status.as_str()
        );
    }

    Ok(())
}

fn display_verbose_sessions(sessions: &[SessionInfo]) -> Result<()> {
    for (i, session) in sessions.iter().enumerate() {
        if i > 0 {
            println!();
        }

        let current_marker = if session.is_current { " (current)" } else { "" };

        println!("Session: {}{}", session.session_id, current_marker);
        println!(
            "  Status: {} {}",
            session.status.symbol(),
            session.status.as_str()
        );
        println!("  Branch: {}", session.branch);
        println!("  Base Branch: {}", session.base_branch);
        println!("  Merge Mode: {}", session.merge_mode);

        if session.status != SessionStatus::Archived {
            println!("  Worktree: {}", session.worktree_path.display());

            if let Some(has_changes) = session.has_uncommitted_changes {
                println!(
                    "  Uncommitted Changes: {}",
                    if has_changes { "yes" } else { "no" }
                );
            }
        }

        if let Some(modified) = session.last_modified {
            println!(
                "  Last Modified: {}",
                modified.format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
    }

    Ok(())
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
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
        state_dir: &std::path::Path,
        session_id: &str,
        branch: &str,
        worktree_path: &str,
        base_branch: &str,
        merge_mode: &str,
    ) -> Result<()> {
        fs::create_dir_all(state_dir)?;

        let state_file = state_dir.join(format!("{}.state", session_id));
        let state_content = format!(
            "{}|{}|{}|{}",
            branch, worktree_path, base_branch, merge_mode
        );
        fs::write(state_file, state_content)?;

        Ok(())
    }

    #[test]
    fn test_session_status_display() {
        assert_eq!(SessionStatus::Active.as_str(), "active");
        assert_eq!(SessionStatus::Dirty.as_str(), "dirty");
        assert_eq!(SessionStatus::Missing.as_str(), "missing");
        assert_eq!(SessionStatus::Archived.as_str(), "archived");

        assert_eq!(SessionStatus::Active.symbol(), "✓");
        assert_eq!(SessionStatus::Dirty.symbol(), "●");
        assert_eq!(SessionStatus::Missing.symbol(), "✗");
        assert_eq!(SessionStatus::Archived.symbol(), "📦");
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("exactly_ten", 11), "exactly_ten");
        assert_eq!(truncate_string("this_is_too_long", 10), "this_is...");
        assert_eq!(truncate_string("abc", 3), "abc");
        assert_eq!(truncate_string("abcd", 3), "...");
    }

    #[test]
    fn test_extract_session_id_from_archived_branch() {
        assert_eq!(
            extract_session_id_from_archived_branch("para/archived/20250609-143052/feature-auth"),
            Some("feature-auth".to_string())
        );

        assert_eq!(
            extract_session_id_from_archived_branch("para/archived/20250609-143052/simple-session"),
            Some("simple-session".to_string())
        );

        assert_eq!(
            extract_session_id_from_archived_branch("regular-branch"),
            None
        );

        assert_eq!(
            extract_session_id_from_archived_branch("para/regular-branch"),
            None
        );
    }

    #[test]
    fn test_list_active_sessions_empty() -> Result<()> {
        let (_temp_dir, git_service) = setup_test_repo();

        let sessions = list_active_sessions(&git_service)?;
        assert!(sessions.is_empty());

        Ok(())
    }

    #[test]
    fn test_list_active_sessions_with_state_files() -> Result<()> {
        let (temp_dir, git_service) = setup_test_repo();
        let repo_root = &git_service.repository().root;
        let state_dir = repo_root.join(".para_state");

        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path)?;

        create_mock_session_state(
            &state_dir,
            "test-session",
            "para/test-branch",
            worktree_path.to_str().unwrap(),
            "master",
            "squash",
        )?;

        let sessions = list_active_sessions(&git_service)?;
        assert_eq!(sessions.len(), 1);

        let session = &sessions[0];
        assert_eq!(session.session_id, "test-session");
        assert_eq!(session.branch, "para/test-branch");
        assert_eq!(session.base_branch, "master");
        assert_eq!(session.merge_mode, "squash");
        assert_eq!(session.status, SessionStatus::Missing); // No worktree exists

        Ok(())
    }

    #[test]
    fn test_parse_session_state_file_valid() -> Result<()> {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let state_file = temp_dir.path().join("test.state");

        let state_content = "para/test-branch|/path/to/worktree|master|squash";
        fs::write(&state_file, state_content)?;

        let worktree_map = HashMap::new();
        let branch_map = HashMap::new();

        let session_info =
            parse_session_state_file(&state_file, "test-session", &worktree_map, &branch_map)?;

        assert_eq!(session_info.session_id, "test-session");
        assert_eq!(session_info.branch, "para/test-branch");
        assert_eq!(
            session_info.worktree_path,
            PathBuf::from("/path/to/worktree")
        );
        assert_eq!(session_info.base_branch, "master");
        assert_eq!(session_info.merge_mode, "squash");
        assert_eq!(session_info.status, SessionStatus::Missing);

        Ok(())
    }

    #[test]
    fn test_parse_session_state_file_invalid_format() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let state_file = temp_dir.path().join("invalid.state");

        let state_content = "invalid|format";
        fs::write(&state_file, state_content).unwrap();

        let worktree_map = HashMap::new();
        let branch_map = HashMap::new();

        let result =
            parse_session_state_file(&state_file, "invalid-session", &worktree_map, &branch_map);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid state file format"));
    }

    #[test]
    fn test_parse_session_state_file_with_defaults() -> Result<()> {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let state_file = temp_dir.path().join("test.state");

        let state_content = "para/test-branch|/path/to/worktree|master";
        fs::write(&state_file, state_content)?;

        let worktree_map = HashMap::new();
        let branch_map = HashMap::new();

        let session_info =
            parse_session_state_file(&state_file, "test-session", &worktree_map, &branch_map)?;

        assert_eq!(session_info.merge_mode, "squash"); // Default value

        Ok(())
    }

    #[test]
    fn test_determine_session_status() -> Result<()> {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let existing_dir = temp_dir.path().join("existing");
        let nonexistent_dir = temp_dir.path().join("nonexistent");

        fs::create_dir_all(&existing_dir)?;

        // Test missing directory
        let status = determine_session_status(&nonexistent_dir, None, None)?;
        assert_eq!(status, SessionStatus::Missing);

        // Test existing directory but no worktree info
        let status = determine_session_status(&existing_dir, None, None)?;
        assert_eq!(status, SessionStatus::Missing);

        // Test with worktree info but directory exists
        let worktree_info = WorktreeInfo {
            path: existing_dir.clone(),
            branch: "test-branch".to_string(),
            commit: "abc123".to_string(),
            is_bare: false,
        };

        let status = determine_session_status(&existing_dir, Some(&worktree_info), None)?;
        assert_eq!(status, SessionStatus::Active);

        Ok(())
    }

    #[test]
    fn test_list_archived_sessions() -> Result<()> {
        let (_temp_dir, git_service) = setup_test_repo();

        // Create some archived branches
        let branch_manager = git_service.branch_manager();

        // Create test branches first
        let current_branch = git_service.get_current_branch()?;
        branch_manager.create_branch("test-branch-1", &current_branch)?;
        branch_manager.create_branch("test-branch-2", &current_branch)?;

        // Switch back to main branch before archiving
        git_service.repository().checkout_branch(&current_branch)?;

        // Archive the branches
        branch_manager.move_to_archive("test-branch-1", "para")?;
        branch_manager.move_to_archive("test-branch-2", "para")?;

        let sessions = list_archived_sessions(&git_service)?;
        assert_eq!(sessions.len(), 2);

        let session_ids: Vec<&str> = sessions.iter().map(|s| s.session_id.as_str()).collect();
        assert!(session_ids.contains(&"test-branch-1"));
        assert!(session_ids.contains(&"test-branch-2"));

        for session in &sessions {
            assert_eq!(session.status, SessionStatus::Archived);
            assert!(session.branch.starts_with("para/archived/"));
        }

        Ok(())
    }

    #[test]
    fn test_execute_no_sessions() -> Result<()> {
        let (_temp_dir, git_service) = setup_test_repo();

        // Mock the discovery to return our test git service
        // We can't easily mock static calls, so we'll test the internal functions directly
        let sessions = list_active_sessions(&git_service)?;
        assert!(sessions.is_empty());

        // Test that empty sessions are handled correctly
        let args = ListArgs {
            verbose: false,
            archived: false,
        };

        let result = display_sessions(&sessions, &args);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_not_in_git_repo() {
        // Test that GitService::discover fails when not in a git repo
        // This test validates the error handling without changing directories
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // We can't easily test the execute function without changing directories
        // So we'll test that GitService::discover_from fails for non-git directories
        let result = GitService::discover_from(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_display_compact_sessions() -> Result<()> {
        let sessions = vec![
            SessionInfo {
                session_id: "test-session-1".to_string(),
                branch: "para/test-branch-1".to_string(),
                worktree_path: PathBuf::from("/path/to/worktree1"),
                base_branch: "master".to_string(),
                merge_mode: "squash".to_string(),
                status: SessionStatus::Active,
                last_modified: None,
                commit_count: None,
                has_uncommitted_changes: Some(false),
                is_current: false,
            },
            SessionInfo {
                session_id: "current-session".to_string(),
                branch: "para/current-branch".to_string(),
                worktree_path: PathBuf::from("/path/to/current"),
                base_branch: "master".to_string(),
                merge_mode: "merge".to_string(),
                status: SessionStatus::Dirty,
                last_modified: None,
                commit_count: None,
                has_uncommitted_changes: Some(true),
                is_current: true,
            },
        ];

        // This should not panic
        let result = display_compact_sessions(&sessions);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_display_verbose_sessions() -> Result<()> {
        let sessions = vec![SessionInfo {
            session_id: "verbose-test".to_string(),
            branch: "para/verbose-branch".to_string(),
            worktree_path: PathBuf::from("/path/to/verbose"),
            base_branch: "develop".to_string(),
            merge_mode: "squash".to_string(),
            status: SessionStatus::Active,
            last_modified: Some(Utc::now()),
            commit_count: Some(5),
            has_uncommitted_changes: Some(false),
            is_current: true,
        }];

        // This should not panic
        let result = display_verbose_sessions(&sessions);
        assert!(result.is_ok());

        Ok(())
    }
}
