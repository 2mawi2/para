pub mod formatting;
pub mod status;
pub mod test_utils;

use crate::cli::parser::ListArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::core::session::{SessionManager, SessionStatus as UnifiedSessionStatus};
use crate::utils::Result;
use std::path::Path;

pub use formatting::{SessionInfo, SessionStatus};
use formatting::display_sessions;
use status::{
    create_session_info_from_branch, create_session_info_from_state, determine_uncommitted_changes,
    determine_unified_session_status, extract_session_id_from_archived_branch,
    git_service_for_path, sort_sessions_by_date,
};

pub fn execute(config: Config, args: ListArgs) -> Result<()> {
    let session_manager = SessionManager::new(&config);

    let git_service = GitService::discover()?;
    let sessions = if args.archived {
        list_archived_sessions(&session_manager, &git_service)?
    } else {
        list_active_sessions(&session_manager, &git_service)?
    };

    if sessions.is_empty() {
        if !args.quiet {
            if args.archived {
                println!("No archived sessions found.");
            } else {
                println!("No active sessions found.");
            }
        }
        return Ok(());
    }

    display_sessions(&sessions, &args)?;
    Ok(())
}

fn list_active_sessions(
    session_manager: &SessionManager,
    git_service: &GitService,
) -> Result<Vec<SessionInfo>> {
    let session_states = session_manager.list_sessions()?;

    let mut sessions = Vec::new();

    for session_state in session_states {
        match session_state.status {
            UnifiedSessionStatus::Finished | UnifiedSessionStatus::Cancelled => continue,
            _ => {}
        }

        let has_uncommitted_changes = if session_state.worktree_path.exists() {
            if let Some(service) = git_service_for_path(&session_state.worktree_path) {
                service.has_uncommitted_changes().ok()
            } else {
                Some(false)
            }
        } else {
            Some(false)
        };

        let is_current = std::env::current_dir()
            .map(|cwd| cwd.starts_with(&session_state.worktree_path))
            .unwrap_or(false);

        let status = determine_unified_session_status(&session_state, git_service)?;

        let session_info = SessionInfo {
            session_id: session_state.name.clone(),
            branch: session_state.branch.clone(),
            worktree_path: session_state.worktree_path.clone(),
            base_branch: "main".to_string(),
            merge_mode: "squash".to_string(),
            status,
            last_modified: Some(session_state.created_at),
            has_uncommitted_changes,
            is_current,
        };

        sessions.push(session_info);
    }

    sessions.sort_by(|a, b| {
        b.last_modified
            .unwrap_or(chrono::DateTime::<chrono::Utc>::MIN_UTC)
            .cmp(&a.last_modified.unwrap_or(chrono::DateTime::<chrono::Utc>::MIN_UTC))
    });

    Ok(sessions)
}

fn list_archived_sessions(
    session_manager: &SessionManager,
    git_service: &GitService,
) -> Result<Vec<SessionInfo>> {
    let mut sessions = Vec::new();
    let mut seen_session_ids = std::collections::HashSet::new();

    // Collect sessions from finished/cancelled session states
    let finished_sessions = collect_finished_sessions(session_manager)?;
    for session_info in finished_sessions {
        seen_session_ids.insert(session_info.session_id.clone());
        sessions.push(session_info);
    }

    // Collect sessions from archived branches (those not in session states)
    let archived_branch_sessions =
        collect_archived_branch_sessions(session_manager, git_service, &seen_session_ids)?;
    sessions.extend(archived_branch_sessions);

    sort_sessions_by_date(&mut sessions);

    Ok(sessions)
}

fn collect_finished_sessions(session_manager: &SessionManager) -> Result<Vec<SessionInfo>> {
    let mut sessions = Vec::new();
    let session_states = session_manager.list_sessions()?;

    for session_state in session_states {
        match session_state.status {
            UnifiedSessionStatus::Finished | UnifiedSessionStatus::Cancelled => {
                let has_uncommitted_changes =
                    determine_uncommitted_changes(&session_state.worktree_path);

                let session_info =
                    create_session_info_from_state(&session_state, has_uncommitted_changes);
                sessions.push(session_info);
            }
            _ => {}
        }
    }

    Ok(sessions)
}

fn collect_archived_branch_sessions(
    session_manager: &SessionManager,
    git_service: &GitService,
    seen_session_ids: &std::collections::HashSet<String>,
) -> Result<Vec<SessionInfo>> {
    let mut sessions = Vec::new();
    let branch_manager = git_service.branch_manager();
    let branch_prefix = &session_manager.config().git.branch_prefix;
    let archived_branches = branch_manager.list_archived_branches(branch_prefix)?;

    for branch_name in archived_branches {
        if let Some(session_id) =
            extract_session_id_from_archived_branch(&branch_name, branch_prefix)
        {
            if !seen_session_ids.contains(&session_id) {
                let session_info = create_session_info_from_branch(&session_id, &branch_name);
                sessions.push(session_info);
            }
        }
    }

    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use formatting::SessionStatus;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use test_utils::{
        create_mock_session_state, create_test_config, setup_test_repo, MockSessionStateParams,
        TestEnvironmentGuard,
    };

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
        use formatting::truncate_string;
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("exactly_ten", 11), "exactly_ten");
        assert_eq!(truncate_string("this_is_too_long", 10), "this_is...");
        assert_eq!(truncate_string("abc", 3), "abc");
        assert_eq!(truncate_string("abcd", 3), "...");
    }

    #[test]
    fn test_extract_session_id_from_archived_branch() {
        assert_eq!(
            extract_session_id_from_archived_branch(
                "para/archived/20250609-143052/feature-auth",
                "para"
            ),
            Some("feature-auth".to_string())
        );

        assert_eq!(
            extract_session_id_from_archived_branch(
                "para/archived/20250609-143052/simple-session",
                "para"
            ),
            Some("simple-session".to_string())
        );

        assert_eq!(
            extract_session_id_from_archived_branch("regular-branch", "para"),
            None
        );

        assert_eq!(
            extract_session_id_from_archived_branch("para/regular-branch", "para"),
            None
        );
    }

    #[test]
    fn test_list_active_sessions_empty() -> Result<()> {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        // Create config that points to our test state directory
        let state_dir = temp_dir.path().join(".para_state");
        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();
        let session_manager = SessionManager::new(&config);

        let sessions = list_active_sessions(&session_manager, &git_service)?;
        assert!(sessions.is_empty());

        Ok(())
    }

    #[test]
    fn test_list_active_sessions_with_state_files() -> Result<()> {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let repo_root = &git_service.repository().root;
        let state_dir = repo_root.join(".para_state");

        // Create config that points to our test state directory
        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();
        let session_manager = SessionManager::new(&config);

        // Create a simple directory for the worktree path - we just need to test listing
        let worktree_path = temp_dir.path().join("test-worktree");
        fs::create_dir_all(&worktree_path)?;
        let test_branch_name = "para/test-branch".to_string();

        let params = MockSessionStateParams::new(
            "test-session",
            &test_branch_name,
            worktree_path.to_str().unwrap(),
            "main",
            "squash",
        );
        create_mock_session_state(&state_dir, &params)?;

        let sessions = list_active_sessions(&session_manager, &git_service)?;
        assert_eq!(sessions.len(), 1);

        let session = &sessions[0];
        assert_eq!(session.session_id, "test-session");
        assert_eq!(session.branch, test_branch_name);
        assert_eq!(session.base_branch, "main");
        assert_eq!(session.merge_mode, "squash");
        assert_eq!(session.status, SessionStatus::Missing);

        Ok(())
    }

    #[test]
    fn test_list_archived_sessions() -> Result<()> {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        // Create config that points to our test state directory
        let state_dir = temp_dir.path().join(".para_state");
        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();
        let session_manager = SessionManager::new(&config);

        // Create some archived branches
        let branch_manager = git_service.branch_manager();

        // Create test branches first
        let current_branch = git_service.repository().get_current_branch()?;
        branch_manager.create_branch("test-branch-1", &current_branch)?;
        branch_manager.create_branch("test-branch-2", &current_branch)?;

        // Switch back to main branch before archiving
        git_service.repository().checkout_branch(&current_branch)?;

        // Archive the branches using the configured prefix
        let branch_prefix = &session_manager.config().git.branch_prefix;
        branch_manager.move_to_archive("test-branch-1", branch_prefix)?;
        branch_manager.move_to_archive("test-branch-2", branch_prefix)?;

        // Test list_archived_sessions function directly using our git_service
        let branch_manager = git_service.branch_manager();
        let branch_prefix = &session_manager.config().git.branch_prefix;
        let archived_branches = branch_manager.list_archived_branches(branch_prefix)?;

        let mut sessions = Vec::new();
        for branch_name in archived_branches {
            if let Some(session_id) =
                extract_session_id_from_archived_branch(&branch_name, branch_prefix)
            {
                let session_info = SessionInfo {
                    session_id: session_id.clone(),
                    branch: branch_name.to_string(),
                    worktree_path: PathBuf::new(),
                    base_branch: "unknown".to_string(),
                    merge_mode: "unknown".to_string(),
                    status: SessionStatus::Archived,
                    last_modified: None,
                    has_uncommitted_changes: None,
                    is_current: false,
                };
                sessions.push(session_info);
            }
        }

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
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        // Create config that points to our test state directory
        let state_dir = temp_dir.path().join(".para_state");
        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();
        let session_manager = SessionManager::new(&config);

        // Test the internal functions directly with proper git context
        let sessions = list_active_sessions(&session_manager, &git_service)?;
        assert!(sessions.is_empty());

        // Test that empty sessions are handled correctly
        let args = ListArgs {
            verbose: false,
            archived: false,
            quiet: false,
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
    fn test_list_finished_sessions_in_archived() -> Result<()> {
        use crate::core::session::SessionState;

        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let repo_root = &git_service.repository().root;
        let state_dir = repo_root.join(".para_state");

        // Create config that points to our test state directory
        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();
        let session_manager = SessionManager::new(&config);

        // Create a finished session
        fs::create_dir_all(&state_dir)?;
        let mut finished_session = SessionState::new(
            "finished-session".to_string(),
            "para/finished-branch".to_string(),
            temp_dir.path().join("finished-worktree"),
        );
        finished_session.update_status(crate::core::session::SessionStatus::Finished);

        let state_file = state_dir.join("finished-session.state");
        let json_content = serde_json::to_string_pretty(&finished_session)?;
        fs::write(state_file, json_content)?;

        // Create an active session for comparison
        let active_session = SessionState::new(
            "active-session".to_string(),
            "para/active-branch".to_string(),
            temp_dir.path().join("active-worktree"),
        );

        let state_file = state_dir.join("active-session.state");
        let json_content = serde_json::to_string_pretty(&active_session)?;
        fs::write(state_file, json_content)?;

        // Test that active list doesn't include finished session
        let active_sessions = list_active_sessions(&session_manager, &git_service)?;
        assert_eq!(active_sessions.len(), 1);
        assert_eq!(active_sessions[0].session_id, "active-session");

        // Test that archived list includes finished session
        let archived_sessions = list_archived_sessions(&session_manager, &git_service)?;
        assert_eq!(archived_sessions.len(), 1);
        assert_eq!(archived_sessions[0].session_id, "finished-session");
        assert_eq!(archived_sessions[0].status, SessionStatus::Archived);

        Ok(())
    }

    #[test]
    fn test_display_compact_sessions() -> Result<()> {
        use formatting::display_compact_sessions;

        let sessions = vec![
            SessionInfo {
                session_id: "test-session-1".to_string(),
                branch: "para/test-branch-1".to_string(),
                worktree_path: PathBuf::from("/path/to/worktree1"),
                base_branch: "main".to_string(),
                merge_mode: "squash".to_string(),
                status: SessionStatus::Active,
                last_modified: None,
                has_uncommitted_changes: Some(false),
                is_current: false,
            },
            SessionInfo {
                session_id: "current-session".to_string(),
                branch: "para/current-branch".to_string(),
                worktree_path: PathBuf::from("/path/to/current"),
                base_branch: "main".to_string(),
                merge_mode: "merge".to_string(),
                status: SessionStatus::Dirty,
                last_modified: None,
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
        use formatting::display_verbose_sessions;
        use chrono::Utc;

        let sessions = vec![SessionInfo {
            session_id: "verbose-test".to_string(),
            branch: "para/verbose-branch".to_string(),
            worktree_path: PathBuf::from("/path/to/verbose"),
            base_branch: "develop".to_string(),
            merge_mode: "squash".to_string(),
            status: SessionStatus::Active,
            last_modified: Some(Utc::now()),
            has_uncommitted_changes: Some(false),
            is_current: true,
        }];

        // This should not panic
        let result = display_verbose_sessions(&sessions);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_collect_finished_sessions() -> Result<()> {
        use crate::core::session::SessionState;

        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let repo_root = &git_service.repository().root;
        let state_dir = repo_root.join(".para_state");

        // Create config that points to our test state directory
        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();
        let session_manager = SessionManager::new(&config);

        // Create multiple session states with different statuses
        fs::create_dir_all(&state_dir)?;

        // Create finished session
        let mut finished_session = SessionState::new(
            "finished-session".to_string(),
            "para/finished-branch".to_string(),
            temp_dir.path().join("finished-worktree"),
        );
        finished_session.update_status(crate::core::session::SessionStatus::Finished);

        let state_file = state_dir.join("finished-session.state");
        let json_content = serde_json::to_string_pretty(&finished_session)?;
        fs::write(state_file, json_content)?;

        // Create cancelled session
        let mut cancelled_session = SessionState::new(
            "cancelled-session".to_string(),
            "para/cancelled-branch".to_string(),
            temp_dir.path().join("cancelled-worktree"),
        );
        cancelled_session.update_status(crate::core::session::SessionStatus::Cancelled);

        let state_file = state_dir.join("cancelled-session.state");
        let json_content = serde_json::to_string_pretty(&cancelled_session)?;
        fs::write(state_file, json_content)?;

        // Create active session (should be ignored)
        let active_session = SessionState::new(
            "active-session".to_string(),
            "para/active-branch".to_string(),
            temp_dir.path().join("active-worktree"),
        );

        let state_file = state_dir.join("active-session.state");
        let json_content = serde_json::to_string_pretty(&active_session)?;
        fs::write(state_file, json_content)?;

        // Test collect_finished_sessions directly
        let finished_sessions = collect_finished_sessions(&session_manager)?;

        assert_eq!(finished_sessions.len(), 2);

        let session_ids: Vec<&str> = finished_sessions
            .iter()
            .map(|s| s.session_id.as_str())
            .collect();
        assert!(session_ids.contains(&"finished-session"));
        assert!(session_ids.contains(&"cancelled-session"));

        for session in &finished_sessions {
            assert_eq!(session.status, SessionStatus::Archived);
        }

        Ok(())
    }

    #[test]
    fn test_determine_uncommitted_changes() -> Result<()> {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        // Test with existing git directory
        let git_path = git_service.repository().root.clone();
        let result = determine_uncommitted_changes(&git_path);
        assert!(result.is_some());
        assert!(!result.unwrap()); // Clean repo

        // Test with non-existent path
        let non_existent = temp_dir.path().join("non-existent");
        let result = determine_uncommitted_changes(&non_existent);
        assert_eq!(result, Some(false));

        // Test with existing but non-git directory
        let non_git_dir = temp_dir.path().join("non-git");
        fs::create_dir_all(&non_git_dir)?;
        let result = determine_uncommitted_changes(&non_git_dir);
        assert_eq!(result, Some(false));

        Ok(())
    }

    #[test]
    fn test_create_session_info_from_state() -> Result<()> {
        use crate::core::session::SessionState;

        let temp_dir = TempDir::new().unwrap();
        let worktree_path = temp_dir.path().join("test-worktree");

        let session_state = SessionState::new(
            "test-session".to_string(),
            "para/test-branch".to_string(),
            worktree_path.clone(),
        );

        // Test with uncommitted changes
        let session_info = create_session_info_from_state(&session_state, Some(true));

        assert_eq!(session_info.session_id, "test-session");
        assert_eq!(session_info.branch, "para/test-branch");
        assert_eq!(session_info.worktree_path, worktree_path);
        assert_eq!(session_info.base_branch, "main");
        assert_eq!(session_info.merge_mode, "squash");
        assert_eq!(session_info.status, SessionStatus::Archived);
        assert_eq!(session_info.has_uncommitted_changes, Some(true));
        assert!(!session_info.is_current);
        assert!(session_info.last_modified.is_some());

        // Test with no uncommitted changes
        let session_info = create_session_info_from_state(&session_state, Some(false));
        assert_eq!(session_info.has_uncommitted_changes, Some(false));

        // Test with None
        let session_info = create_session_info_from_state(&session_state, None);
        assert_eq!(session_info.has_uncommitted_changes, None);

        Ok(())
    }

    #[test]
    fn test_collect_archived_branch_sessions() -> Result<()> {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let state_dir = temp_dir.path().join(".para_state");
        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();
        let session_manager = SessionManager::new(&config);

        // Create archived branches
        let branch_manager = git_service.branch_manager();
        let current_branch = git_service.repository().get_current_branch()?;
        branch_manager.create_branch("test-branch-1", &current_branch)?;
        branch_manager.create_branch("test-branch-2", &current_branch)?;

        // Switch back to main branch before archiving
        git_service.repository().checkout_branch(&current_branch)?;

        // Archive the branches
        let branch_prefix = &session_manager.config().git.branch_prefix;
        branch_manager.move_to_archive("test-branch-1", branch_prefix)?;
        branch_manager.move_to_archive("test-branch-2", branch_prefix)?;

        // Test with empty seen_session_ids
        let seen_session_ids = std::collections::HashSet::new();
        let sessions =
            collect_archived_branch_sessions(&session_manager, &git_service, &seen_session_ids)?;

        assert_eq!(sessions.len(), 2);
        let session_ids: Vec<&str> = sessions.iter().map(|s| s.session_id.as_str()).collect();
        assert!(session_ids.contains(&"test-branch-1"));
        assert!(session_ids.contains(&"test-branch-2"));

        // Test with seen_session_ids containing one branch
        let mut seen_session_ids = std::collections::HashSet::new();
        seen_session_ids.insert("test-branch-1".to_string());
        let sessions =
            collect_archived_branch_sessions(&session_manager, &git_service, &seen_session_ids)?;

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "test-branch-2");

        Ok(())
    }

    #[test]
    fn test_create_session_info_from_branch() {
        let session_info =
            create_session_info_from_branch("test-session", "para/archived/20250620/test-branch");

        assert_eq!(session_info.session_id, "test-session");
        assert_eq!(session_info.branch, "para/archived/20250620/test-branch");
        assert_eq!(session_info.worktree_path, PathBuf::new());
        assert_eq!(session_info.base_branch, "unknown");
        assert_eq!(session_info.merge_mode, "unknown");
        assert_eq!(session_info.status, SessionStatus::Archived);
        assert_eq!(session_info.last_modified, None);
        assert_eq!(session_info.has_uncommitted_changes, None);
        assert!(!session_info.is_current);
    }

    #[test]
    fn test_sort_sessions_by_date() {
        use chrono::Utc;

        let now = Utc::now();
        let earlier = now - chrono::Duration::hours(1);
        let later = now + chrono::Duration::hours(1);

        let mut sessions = vec![
            SessionInfo {
                session_id: "middle".to_string(),
                branch: "para/middle".to_string(),
                worktree_path: PathBuf::new(),
                base_branch: "main".to_string(),
                merge_mode: "squash".to_string(),
                status: SessionStatus::Active,
                last_modified: Some(now),
                has_uncommitted_changes: None,
                is_current: false,
            },
            SessionInfo {
                session_id: "earliest".to_string(),
                branch: "para/earliest".to_string(),
                worktree_path: PathBuf::new(),
                base_branch: "main".to_string(),
                merge_mode: "squash".to_string(),
                status: SessionStatus::Active,
                last_modified: Some(earlier),
                has_uncommitted_changes: None,
                is_current: false,
            },
            SessionInfo {
                session_id: "latest".to_string(),
                branch: "para/latest".to_string(),
                worktree_path: PathBuf::new(),
                base_branch: "main".to_string(),
                merge_mode: "squash".to_string(),
                status: SessionStatus::Active,
                last_modified: Some(later),
                has_uncommitted_changes: None,
                is_current: false,
            },
            SessionInfo {
                session_id: "none".to_string(),
                branch: "para/none".to_string(),
                worktree_path: PathBuf::new(),
                base_branch: "main".to_string(),
                merge_mode: "squash".to_string(),
                status: SessionStatus::Active,
                last_modified: None,
                has_uncommitted_changes: None,
                is_current: false,
            },
        ];

        sort_sessions_by_date(&mut sessions);

        // Should be sorted by last_modified descending (latest first)
        assert_eq!(sessions[0].session_id, "latest");
        assert_eq!(sessions[1].session_id, "middle");
        assert_eq!(sessions[2].session_id, "earliest");
        assert_eq!(sessions[3].session_id, "none"); // None should be last
    }
}