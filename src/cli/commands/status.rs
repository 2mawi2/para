use crate::cli::parser::{StatusArgs, StatusCommands};
use crate::config::Config;
use crate::core::session::SessionManager;
use crate::core::status::Status;
use crate::utils::{ParaError, Result};
use std::path::PathBuf;

pub fn execute(config: Config, args: StatusArgs) -> Result<()> {
    match args.command {
        Some(StatusCommands::Show { session, json }) => show_status(config, session, json),
        None => {
            // Handle the original update status functionality
            update_status(config, args)
        }
    }
}

fn update_status(config: Config, args: StatusArgs) -> Result<()> {
    // Validate required arguments for update
    let task = args.task.ok_or_else(|| {
        ParaError::invalid_args("Task description is required when updating status")
    })?;

    let tests = args.tests.ok_or_else(|| {
        ParaError::invalid_args("Test status (--tests) is required when updating status")
    })?;

    let confidence = args.confidence.ok_or_else(|| {
        ParaError::invalid_args("Confidence level (--confidence) is required when updating status")
    })?;

    // Detect session from current directory or use provided session name
    let session_manager = SessionManager::new(&config);

    let session_name = match args.session {
        Some(name) => name,
        None => {
            // Try to detect session from current directory
            let current_dir = std::env::current_dir().map_err(|e| {
                ParaError::fs_error(format!("Failed to get current directory: {}", e))
            })?;

            match session_manager.find_session_by_path(&current_dir)? {
                Some(session) => session.name,
                None => {
                    return Err(ParaError::invalid_args(
                        "Not in a para session directory. Use --session to specify session name.",
                    ));
                }
            }
        }
    };

    // Verify session exists
    if !session_manager.session_exists(&session_name) {
        return Err(ParaError::session_not_found(&session_name));
    }

    // Parse and validate arguments
    let test_status =
        Status::parse_test_status(&tests).map_err(|e| ParaError::invalid_args(e.to_string()))?;
    let confidence_level = Status::parse_confidence(&confidence)
        .map_err(|e| ParaError::invalid_args(e.to_string()))?;

    // Create status object
    let mut status = Status::new(session_name.clone(), task, test_status, confidence_level);

    // Handle optional todos
    if let Some(todos_str) = args.todos {
        let (completed, total) =
            Status::parse_todos(&todos_str).map_err(|e| ParaError::invalid_args(e.to_string()))?;
        status = status.with_todos(completed, total);
    }

    // Handle blocked state
    if args.blocked {
        // If blocked, use the task description as the blocked reason
        let task_description = status.current_task.clone();
        status = status.with_blocked(Some(task_description));
    }

    // Save status to file
    let state_dir = PathBuf::from(&config.directories.state_dir);
    status
        .save(&state_dir)
        .map_err(|e| ParaError::config_error(e.to_string()))?;

    println!("Status updated for session '{}'", session_name);

    Ok(())
}

fn show_status(config: Config, session: Option<String>, json: bool) -> Result<()> {
    let state_dir = PathBuf::from(&config.directories.state_dir);
    let session_manager = SessionManager::new(&config);

    match session {
        Some(session_name) => {
            // Show specific session status
            let status = Status::load(&state_dir, &session_name)
                .map_err(|e| ParaError::config_error(e.to_string()))?;

            match status {
                Some(s) => {
                    if json {
                        let json_str = serde_json::to_string_pretty(&s).map_err(|e| {
                            ParaError::config_error(format!("Failed to serialize status: {}", e))
                        })?;
                        println!("{}", json_str);
                    } else {
                        display_status(&s);
                    }
                }
                None => {
                    if !json {
                        println!("No status found for session '{}'", session_name);
                    }
                }
            }
        }
        None => {
            // Show all session statuses
            let sessions = session_manager.list_sessions()?;
            let mut statuses = Vec::new();

            for session_state in sessions {
                if let Some(status) = Status::load(&state_dir, &session_state.name)
                    .map_err(|e| ParaError::config_error(e.to_string()))?
                {
                    statuses.push(status);
                }
            }

            if json {
                let json_str = serde_json::to_string_pretty(&statuses).map_err(|e| {
                    ParaError::config_error(format!("Failed to serialize status: {}", e))
                })?;
                println!("{}", json_str);
            } else if statuses.is_empty() {
                println!("No session statuses found.");
            } else {
                display_all_statuses(&statuses);
            }
        }
    }

    Ok(())
}

fn display_status(status: &Status) {
    println!("Session: {}", status.session_name);
    println!("Task: {}", status.current_task);
    println!("Tests: {}", status.test_status);
    println!("Confidence: {}", status.confidence);

    if let Some(todos) = status.format_todos() {
        println!("Progress: {}", todos);
    }

    if status.is_blocked {
        println!("Status: BLOCKED");
        if let Some(reason) = &status.blocked_reason {
            println!("Reason: {}", reason);
        }
    }

    println!(
        "Last Update: {}",
        status.last_update.format("%Y-%m-%d %H:%M:%S UTC")
    );
}

fn display_all_statuses(statuses: &[Status]) {
    // Sort by last update time (most recent first)
    let mut sorted_statuses = statuses.to_vec();
    sorted_statuses.sort_by(|a, b| b.last_update.cmp(&a.last_update));

    println!(
        "{:<20} {:<40} {:<10} {:<10} {:<15} {:<10}",
        "Session", "Current Task", "Tests", "Confidence", "Progress", "Status"
    );
    println!("{}", "-".repeat(110));

    for status in sorted_statuses {
        let task = if status.current_task.len() > 38 {
            format!("{}...", &status.current_task[..35])
        } else {
            status.current_task.clone()
        };

        let progress = status.format_todos().unwrap_or_else(|| "-".to_string());
        let status_str = if status.is_blocked {
            "BLOCKED"
        } else {
            "Active"
        };

        println!(
            "{:<20} {:<40} {:<10} {:<10} {:<15} {:<10}",
            status.session_name,
            task,
            status.test_status.to_string(),
            status.confidence.to_string(),
            progress,
            status_str
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Local test helper functions (similar to list.rs pattern)
    fn create_test_config() -> crate::config::Config {
        crate::config::defaults::default_config()
    }

    fn setup_test_repo() -> (TempDir, crate::core::git::GitService) {
        use std::fs;
        use std::process::Command;

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

        let service = crate::core::git::GitService::discover_from(repo_path)
            .expect("Failed to discover repo");
        (temp_dir, service)
    }

    struct TestEnvironmentGuard {
        original_dir: std::path::PathBuf,
        original_home: String,
    }

    impl TestEnvironmentGuard {
        fn new(
            git_temp: &TempDir,
            temp_dir: &TempDir,
        ) -> std::result::Result<Self, std::io::Error> {
            let original_dir = std::env::current_dir().unwrap_or_else(|_| {
                git_temp
                    .path()
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("/tmp"))
                    .to_path_buf()
            });

            std::env::set_current_dir(git_temp.path())?;

            let original_home = std::env::var("HOME").unwrap_or_default();
            std::env::set_var("HOME", temp_dir.path());

            Ok(TestEnvironmentGuard {
                original_dir,
                original_home,
            })
        }
    }

    impl Drop for TestEnvironmentGuard {
        fn drop(&mut self) {
            if let Err(_e) = std::env::set_current_dir(&self.original_dir) {
                let _ = std::env::set_current_dir("/tmp");
            }

            if !self.original_home.is_empty() {
                std::env::set_var("HOME", &self.original_home);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }

    #[test]
    fn test_status_update_with_session_name() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, _git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Create a test session
        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "test-session".to_string(),
            "test/branch".to_string(),
            temp_dir.path().join("worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        // Update status
        let args = StatusArgs {
            command: None,
            task: Some("Working on tests".to_string()),
            tests: Some("passed".to_string()),
            confidence: Some("high".to_string()),
            todos: Some("3/5".to_string()),
            blocked: false,
            session: Some("test-session".to_string()),
        };

        let result = execute(config.clone(), args);
        assert!(result.is_ok());

        // Verify status was saved
        let state_dir = PathBuf::from(&config.directories.state_dir);
        let loaded_status = Status::load(&state_dir, "test-session").unwrap();
        assert!(loaded_status.is_some());

        let status = loaded_status.unwrap();
        assert_eq!(status.current_task, "Working on tests");
        assert_eq!(status.test_status, crate::core::status::TestStatus::Passed);
        assert_eq!(
            status.confidence,
            crate::core::status::ConfidenceLevel::High
        );
        assert_eq!(status.todos_completed, Some(3));
        assert_eq!(status.todos_total, Some(5));
        assert!(!status.is_blocked);
    }

    #[test]
    fn test_status_update_with_blocked() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, _git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Create a test session
        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "blocked-session".to_string(),
            "test/branch".to_string(),
            temp_dir.path().join("worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        // Update status with blocked flag
        let args = StatusArgs {
            command: None,
            task: Some("Need help with Redis mocking".to_string()),
            tests: Some("failed".to_string()),
            confidence: Some("low".to_string()),
            todos: None,
            blocked: true,
            session: Some("blocked-session".to_string()),
        };

        let result = execute(config.clone(), args);
        assert!(result.is_ok());

        // Verify blocked status
        let state_dir = PathBuf::from(&config.directories.state_dir);
        let status = Status::load(&state_dir, "blocked-session")
            .unwrap()
            .unwrap();
        assert!(status.is_blocked);
        assert_eq!(
            status.blocked_reason,
            Some("Need help with Redis mocking".to_string())
        );
    }

    #[test]
    fn test_status_update_context_detection() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, _git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Create a test session with worktree
        let worktree_path = temp_dir.path().join("test-worktree");
        std::fs::create_dir_all(&worktree_path).unwrap();

        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "context-session".to_string(),
            "test/branch".to_string(),
            worktree_path.clone(),
        );
        session_manager.save_state(&session_state).unwrap();

        // Change to worktree directory
        std::env::set_current_dir(&worktree_path).unwrap();

        // Update status without specifying session (should auto-detect)
        let args = StatusArgs {
            command: None,
            task: Some("Auto-detected session".to_string()),
            tests: Some("unknown".to_string()),
            confidence: Some("medium".to_string()),
            todos: None,
            blocked: false,
            session: None,
        };

        let result = execute(config.clone(), args);
        assert!(result.is_ok());

        // Verify status was saved for the correct session
        let state_dir = PathBuf::from(&config.directories.state_dir);
        let status = Status::load(&state_dir, "context-session")
            .unwrap()
            .unwrap();
        assert_eq!(status.current_task, "Auto-detected session");
    }

    #[test]
    fn test_status_show_single_session() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, _git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Create a test session and status
        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "show-test".to_string(),
            "test/branch".to_string(),
            temp_dir.path().join("worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        // Create a status
        let status = Status::new(
            "show-test".to_string(),
            "Testing show command".to_string(),
            crate::core::status::TestStatus::Passed,
            crate::core::status::ConfidenceLevel::High,
        );
        let state_dir = PathBuf::from(&config.directories.state_dir);
        status.save(&state_dir).unwrap();

        // Show status
        let args = StatusArgs {
            command: Some(StatusCommands::Show {
                session: Some("show-test".to_string()),
                json: false,
            }),
            task: None,
            tests: None,
            confidence: None,
            todos: None,
            blocked: false,
            session: None,
        };

        let result = execute(config, args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_show_all_sessions() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, _git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        let session_manager = SessionManager::new(&config);
        let state_dir = PathBuf::from(&config.directories.state_dir);

        // Create multiple sessions with statuses
        for i in 1..=3 {
            let session_name = format!("session-{}", i);
            let session_state = crate::core::session::SessionState::new(
                session_name.clone(),
                format!("test/branch-{}", i),
                temp_dir.path().join(format!("worktree-{}", i)),
            );
            session_manager.save_state(&session_state).unwrap();

            let status = Status::new(
                session_name,
                format!("Working on feature {}", i),
                crate::core::status::TestStatus::Unknown,
                crate::core::status::ConfidenceLevel::Medium,
            );
            status.save(&state_dir).unwrap();
        }

        // Show all statuses
        let args = StatusArgs {
            command: Some(StatusCommands::Show {
                session: None,
                json: false,
            }),
            task: None,
            tests: None,
            confidence: None,
            todos: None,
            blocked: false,
            session: None,
        };

        let result = execute(config, args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_update_invalid_session() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, _git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        let args = StatusArgs {
            command: None,
            task: Some("Should fail".to_string()),
            tests: Some("passed".to_string()),
            confidence: Some("high".to_string()),
            todos: None,
            blocked: false,
            session: Some("nonexistent-session".to_string()),
        };

        let result = execute(config, args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Session 'nonexistent-session' not found"));
    }

    #[test]
    fn test_status_update_invalid_test_status() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, _git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Create a test session
        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "test-session".to_string(),
            "test/branch".to_string(),
            temp_dir.path().join("worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        let args = StatusArgs {
            command: None,
            task: Some("Test task".to_string()),
            tests: Some("invalid".to_string()),
            confidence: Some("high".to_string()),
            todos: None,
            blocked: false,
            session: Some("test-session".to_string()),
        };

        let result = execute(config, args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Test status must be"));
    }

    #[test]
    fn test_status_update_invalid_todos() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, _git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Create a test session
        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "test-session".to_string(),
            "test/branch".to_string(),
            temp_dir.path().join("worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        let args = StatusArgs {
            command: None,
            task: Some("Test task".to_string()),
            tests: Some("passed".to_string()),
            confidence: Some("high".to_string()),
            todos: Some("invalid-format".to_string()),
            blocked: false,
            session: Some("test-session".to_string()),
        };

        let result = execute(config, args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Todos must be in format"));
    }

    #[test]
    fn test_status_update_missing_required_args() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, _git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Missing task
        let args = StatusArgs {
            command: None,
            task: None,
            tests: Some("passed".to_string()),
            confidence: Some("high".to_string()),
            todos: None,
            blocked: false,
            session: Some("test-session".to_string()),
        };

        let result = execute(config.clone(), args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Task description is required"));

        // Missing tests
        let args = StatusArgs {
            command: None,
            task: Some("Test task".to_string()),
            tests: None,
            confidence: Some("high".to_string()),
            todos: None,
            blocked: false,
            session: Some("test-session".to_string()),
        };

        let result = execute(config.clone(), args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Test status (--tests) is required"));

        // Missing confidence
        let args = StatusArgs {
            command: None,
            task: Some("Test task".to_string()),
            tests: Some("passed".to_string()),
            confidence: None,
            todos: None,
            blocked: false,
            session: Some("test-session".to_string()),
        };

        let result = execute(config, args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Confidence level (--confidence) is required"));
    }

    #[test]
    fn test_display_status_formatting() {
        // Test basic status display
        let status = Status::new(
            "test-session".to_string(),
            "Working on authentication".to_string(),
            crate::core::status::TestStatus::Passed,
            crate::core::status::ConfidenceLevel::High,
        );

        // We can't easily test println! output, but we can test the logic
        // by verifying the status fields are accessible and formatted correctly
        assert_eq!(status.session_name, "test-session");
        assert_eq!(status.current_task, "Working on authentication");
        assert_eq!(status.test_status, crate::core::status::TestStatus::Passed);
        assert_eq!(
            status.confidence,
            crate::core::status::ConfidenceLevel::High
        );
        assert!(!status.is_blocked);
        assert!(status.format_todos().is_none());

        // Test with todos
        let status_with_todos = status.clone().with_todos(3, 7);
        assert_eq!(
            status_with_todos.format_todos(),
            Some("43% (3/7)".to_string())
        );

        // Test with blocked status
        let status_blocked = status.with_blocked(Some("Need help with Redis".to_string()));
        assert!(status_blocked.is_blocked);
        assert_eq!(
            status_blocked.blocked_reason,
            Some("Need help with Redis".to_string())
        );
    }

    #[test]
    fn test_display_all_statuses_sorting() {
        use chrono::{Duration, Utc};

        let now = Utc::now();

        // Create statuses with different update times
        let mut status1 = Status::new(
            "session1".to_string(),
            "Task 1".to_string(),
            crate::core::status::TestStatus::Passed,
            crate::core::status::ConfidenceLevel::High,
        );
        status1.last_update = now - Duration::hours(2); // 2 hours ago

        let mut status2 = Status::new(
            "session2".to_string(),
            "Task 2".to_string(),
            crate::core::status::TestStatus::Failed,
            crate::core::status::ConfidenceLevel::Low,
        );
        status2.last_update = now - Duration::minutes(30); // 30 minutes ago

        let mut status3 = Status::new(
            "session3".to_string(),
            "Task 3".to_string(),
            crate::core::status::TestStatus::Unknown,
            crate::core::status::ConfidenceLevel::Medium,
        );
        status3.last_update = now; // now

        let statuses = vec![status1, status2, status3];

        // Test the sorting logic that display_all_statuses uses
        let mut sorted_statuses = statuses.clone();
        sorted_statuses.sort_by(|a, b| b.last_update.cmp(&a.last_update));

        // Should be sorted by most recent first
        assert_eq!(sorted_statuses[0].session_name, "session3"); // now
        assert_eq!(sorted_statuses[1].session_name, "session2"); // 30 min ago
        assert_eq!(sorted_statuses[2].session_name, "session1"); // 2 hours ago
    }

    #[test]
    fn test_display_status_task_truncation() {
        // Test the task truncation logic in display_all_statuses
        let long_task = "This is a very long task description that should be truncated because it exceeds the maximum length allowed for display in the table format";

        let status = Status::new(
            "session-long-task".to_string(),
            long_task.to_string(),
            crate::core::status::TestStatus::Passed,
            crate::core::status::ConfidenceLevel::High,
        );

        // Test truncation logic (mimicking what display_all_statuses does)
        let task = if status.current_task.len() > 38 {
            format!("{}...", &status.current_task[..35])
        } else {
            status.current_task.clone()
        };

        assert!(task.len() <= 38); // 35 chars + "..."
        assert!(task.ends_with("..."));
        assert_eq!(task, "This is a very long task descriptio...");
    }

    #[test]
    fn test_display_status_blocked_formatting() {
        let status = Status::new(
            "blocked-session".to_string(),
            "Stuck on Redis configuration".to_string(),
            crate::core::status::TestStatus::Failed,
            crate::core::status::ConfidenceLevel::Low,
        )
        .with_blocked(Some("Need help with Redis mocking".to_string()));

        // Test blocked status formatting logic
        let status_str = if status.is_blocked {
            "BLOCKED"
        } else {
            "Active"
        };

        assert_eq!(status_str, "BLOCKED");
        assert!(status.is_blocked);
        assert_eq!(
            status.blocked_reason,
            Some("Need help with Redis mocking".to_string())
        );
    }

    #[test]
    fn test_display_status_progress_formatting() {
        // Test various todo progress scenarios
        let status_no_todos = Status::new(
            "session1".to_string(),
            "Task without todos".to_string(),
            crate::core::status::TestStatus::Passed,
            crate::core::status::ConfidenceLevel::High,
        );
        assert_eq!(
            status_no_todos
                .format_todos()
                .unwrap_or_else(|| "-".to_string()),
            "-"
        );

        let status_with_todos = Status::new(
            "session2".to_string(),
            "Task with todos".to_string(),
            crate::core::status::TestStatus::Passed,
            crate::core::status::ConfidenceLevel::High,
        )
        .with_todos(3, 5);
        assert_eq!(
            status_with_todos.format_todos(),
            Some("60% (3/5)".to_string())
        );

        let status_complete = Status::new(
            "session3".to_string(),
            "Complete task".to_string(),
            crate::core::status::TestStatus::Passed,
            crate::core::status::ConfidenceLevel::High,
        )
        .with_todos(5, 5);
        assert_eq!(
            status_complete.format_todos(),
            Some("100% (5/5)".to_string())
        );

        let status_zero_todos = Status::new(
            "session4".to_string(),
            "No todos done".to_string(),
            crate::core::status::TestStatus::Failed,
            crate::core::status::ConfidenceLevel::Low,
        )
        .with_todos(0, 3);
        assert_eq!(
            status_zero_todos.format_todos(),
            Some("0% (0/3)".to_string())
        );
    }
}
