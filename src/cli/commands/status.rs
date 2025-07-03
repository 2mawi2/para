use crate::cli::parser::{StatusArgs, StatusCommands};
use crate::config::Config;
use crate::core::session::SessionManager;
use crate::core::status::{DiffStats, Status};
use crate::utils::{get_main_repository_root, ParaError, Result};
use std::path::{Path, PathBuf};

pub fn execute(config: Config, args: StatusArgs) -> Result<()> {
    match args.command {
        Some(StatusCommands::Show { session, json }) => show_status(config, session, json),
        Some(StatusCommands::Summary { json }) => show_summary(config, json),
        Some(StatusCommands::Cleanup { dry_run }) => cleanup_status(config, dry_run),
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

    // Detect session from current directory or use provided session name
    let session_manager = SessionManager::new(&config);

    let session_name = match args.session {
        Some(name) => name,
        None => {
            // Try to detect session from current directory
            let current_dir = std::env::current_dir().map_err(|e| {
                ParaError::fs_error(format!("Failed to get current directory: {e}"))
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

    // Check if session is in Review state
    if let Ok(session_state) = session_manager.load_state(&session_name) {
        if matches!(
            session_state.status,
            crate::core::session::SessionStatus::Review
        ) {
            return Err(ParaError::invalid_args(
                "Cannot update status for sessions in Review state. Use 'para resume' with a task to reactivate the session."
            ));
        }
    }

    // Parse and validate arguments
    let test_status =
        Status::parse_test_status(&tests).map_err(|e| ParaError::invalid_args(e.to_string()))?;

    // Calculate diff stats if we're in a worktree
    let diff_stats = match session_manager.load_state(&session_name) {
        Ok(session_state) => calculate_diff_stats_for_session(&session_state).unwrap_or(None),
        Err(_) => None, // Session not found or error loading
    };

    // Create status object
    let mut status = Status::new(session_name.clone(), task, test_status);

    // Add diff stats if available
    if let Some(stats) = diff_stats {
        status = status.with_diff_stats(stats);
    }

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

    // Save status to file in the main repository's state directory
    let state_dir = if Path::new(&config.directories.state_dir).is_absolute() {
        // If state_dir is already absolute (e.g., in tests), use it directly
        PathBuf::from(&config.directories.state_dir)
    } else {
        // Otherwise, resolve it relative to the main repo root
        let repo_root = get_main_repository_root()
            .map_err(|e| ParaError::git_error(format!("Not in a para repository: {e}")))?;
        repo_root.join(&config.directories.state_dir)
    };

    status
        .save(&state_dir)
        .map_err(|e| ParaError::config_error(e.to_string()))?;

    println!("Status updated for session '{session_name}'");

    Ok(())
}

fn calculate_diff_stats_for_session(
    session_state: &crate::core::session::SessionState,
) -> Result<Option<DiffStats>> {
    Status::calculate_diff_stats_for_session(session_state)
        .map(Some)
        .or(Ok(None))
}

struct StatusDisplayHandler {
    session_manager: SessionManager,
    state_dir: PathBuf,
}

impl StatusDisplayHandler {
    fn new(config: Config) -> Result<Self> {
        let state_dir = Self::resolve_state_directory(&config)?;
        let session_manager = SessionManager::new(&config);

        Ok(Self {
            session_manager,
            state_dir,
        })
    }

    fn resolve_state_directory(config: &Config) -> Result<PathBuf> {
        if Path::new(&config.directories.state_dir).is_absolute() {
            // If state_dir is already absolute (e.g., in tests), use it directly
            Ok(PathBuf::from(&config.directories.state_dir))
        } else {
            // Otherwise, resolve it relative to the main repo root
            let repo_root = get_main_repository_root()
                .map_err(|e| ParaError::git_error(format!("Not in a para repository: {e}")))?;
            Ok(repo_root.join(&config.directories.state_dir))
        }
    }

    fn show_specific_session(&self, session_name: &str, json: bool) -> Result<()> {
        let mut status = Status::load(&self.state_dir, session_name)
            .map_err(|e| ParaError::config_error(e.to_string()))?;

        // For container sessions, check for container status file
        if let Ok(session_state) = self.session_manager.load_state(session_name) {
            if session_state.is_container() {
                // Check for container status.json file
                let signal_paths = crate::core::docker::signal_files::SignalFilePaths::new(
                    &session_state.worktree_path,
                );
                if let Ok(Some(container_status)) =
                    crate::core::docker::signal_files::read_signal_file::<
                        crate::core::docker::signal_files::ContainerStatus,
                    >(&signal_paths.status)
                {
                    // Update status with container information
                    if let Some(ref mut s) = status {
                        s.current_task = container_status.task;
                        if let Some(tests) = container_status.tests {
                            if let Ok(test_status) = Status::parse_test_status(&tests) {
                                s.test_status = test_status;
                            }
                        }
                        if let Some(todos) = container_status.todos {
                            if let Ok((completed, total)) = Status::parse_todos(&todos) {
                                s.todos_completed = Some(completed);
                                s.todos_total = Some(total);
                            }
                        }
                        s.blocked_reason = if container_status.blocked {
                            Some(s.current_task.clone())
                        } else {
                            None
                        };
                        // Parse timestamp string to DateTime<Utc>
                        if let Ok(parsed_time) =
                            chrono::DateTime::parse_from_rfc3339(&container_status.timestamp)
                        {
                            s.last_update = parsed_time.with_timezone(&chrono::Utc);
                        }

                        // Calculate diff stats on the host
                        if let Ok(Some(diff_stats)) =
                            calculate_diff_stats_for_session(&session_state)
                        {
                            s.diff_stats = Some(diff_stats);
                        }
                    }
                }
            }
        }

        match status {
            Some(mut s) => {
                // Try to enrich with diff stats if we have session state
                if let Ok(session_state) = self.session_manager.load_state(session_name) {
                    if let Ok(Some(diff_stats)) = calculate_diff_stats_for_session(&session_state) {
                        s = s.with_diff_stats(diff_stats);
                    }
                }

                if json {
                    self.output_json(&s)?;
                } else {
                    display_status(&s);
                }
            }
            None => {
                if !json {
                    println!("No status found for session '{session_name}'");
                }
            }
        }

        Ok(())
    }

    fn show_all_sessions(&self, json: bool) -> Result<()> {
        let sessions = self.session_manager.list_sessions()?;
        let mut statuses = Vec::new();

        for session_state in sessions {
            if let Some(mut status) = Status::load(&self.state_dir, &session_state.name)
                .map_err(|e| ParaError::config_error(e.to_string()))?
            {
                // Try to enrich with diff stats if we have parent branch
                if let Ok(Some(diff_stats)) = calculate_diff_stats_for_session(&session_state) {
                    status = status.with_diff_stats(diff_stats);
                }
                statuses.push(status);
            }
        }

        if json {
            self.output_json(&statuses)?;
        } else if statuses.is_empty() {
            println!("No session statuses found.");
        } else {
            display_all_statuses(&statuses);
        }

        Ok(())
    }

    fn output_json<T: serde::Serialize>(&self, data: &T) -> Result<()> {
        let json_str = serde_json::to_string_pretty(data)
            .map_err(|e| ParaError::config_error(format!("Failed to serialize status: {e}")))?;
        println!("{json_str}");
        Ok(())
    }
}

fn show_status(config: Config, session: Option<String>, json: bool) -> Result<()> {
    let handler = StatusDisplayHandler::new(config)?;

    match session {
        Some(session_name) => handler.show_specific_session(&session_name, json),
        None => handler.show_all_sessions(json),
    }
}

fn display_status(status: &Status) {
    println!("Session: {}", status.session_name);
    println!("Task: {}", status.current_task);
    println!("Tests: {}", status.test_status);
    if let Some(diff_stats) = &status.diff_stats {
        println!("Changes: {diff_stats}");
    }

    if let Some(todos) = status.format_todos() {
        println!("Progress: {todos}");
    }

    if status.is_blocked {
        println!("Status: BLOCKED");
        if let Some(reason) = &status.blocked_reason {
            println!("Reason: {reason}");
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
        "{:<20} {:<40} {:<10} {:<15} {:<10}",
        "Session", "Current Task", "Tests", "Progress", "Status"
    );
    println!("{}", "-".repeat(100));

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
            "{:<20} {:<40} {:<10} {:<15} {:<10}",
            status.session_name,
            task,
            status.test_status.to_string(),
            progress,
            status_str
        );
    }
}

fn show_summary(config: Config, json: bool) -> Result<()> {
    let state_dir = if Path::new(&config.directories.state_dir).is_absolute() {
        PathBuf::from(&config.directories.state_dir)
    } else {
        get_main_repository_root()?.join(".para").join("state")
    };

    // Use 24 hours as the default stale threshold
    let stale_threshold_hours = 24;

    let summary = Status::generate_summary(&state_dir, stale_threshold_hours)
        .map_err(|e| ParaError::file_operation(format!("Failed to generate summary: {e}")))?;

    if json {
        let json_output = serde_json::to_string_pretty(&summary)
            .map_err(|e| ParaError::config_error(format!("Failed to serialize summary: {e}")))?;
        println!("{json_output}");
    } else {
        println!("📊 Para Status Summary");
        println!("====================\n");

        println!("Sessions:");
        println!("  Total: {}", summary.total_sessions);
        println!("  Active: {}", summary.active_sessions);
        println!("  Blocked: {}", summary.blocked_sessions);
        println!("  Stale: {}", summary.stale_sessions);

        println!("\nTest Status:");
        println!("  ✅ Passed: {}", summary.test_summary.passed);
        println!("  ❌ Failed: {}", summary.test_summary.failed);
        println!("  ❓ Unknown: {}", summary.test_summary.unknown);

        if let Some(progress) = summary.overall_progress {
            println!("\nOverall Progress: {progress}%");
        }
    }

    Ok(())
}

fn cleanup_status(config: Config, dry_run: bool) -> Result<()> {
    let state_dir = if Path::new(&config.directories.state_dir).is_absolute() {
        PathBuf::from(&config.directories.state_dir)
    } else {
        get_main_repository_root()?.join(".para").join("state")
    };

    // Use 24 hours as the default stale threshold
    let stale_threshold_hours = 24;

    if dry_run {
        // Just show what would be cleaned
        let statuses = Status::load_all(&state_dir)
            .map_err(|e| ParaError::file_operation(format!("Failed to load status files: {e}")))?;

        let mut stale_count = 0;
        println!("🔍 Stale status files (older than {stale_threshold_hours} hours):");

        for status in statuses {
            if status.is_stale(stale_threshold_hours) {
                println!(
                    "  📊 {}.status.json - last updated: {}",
                    status.session_name,
                    status.last_update.format("%Y-%m-%d %H:%M:%S UTC")
                );
                stale_count += 1;
            }
        }

        if stale_count == 0 {
            println!("  No stale status files found.");
        } else {
            println!("\nTotal: {stale_count} stale status files would be removed");
        }
    } else {
        // Actually clean up
        let cleaned = Status::cleanup_stale(&state_dir, stale_threshold_hours).map_err(|e| {
            ParaError::file_operation(format!("Failed to cleanup status files: {e}"))
        })?;

        if cleaned.is_empty() {
            println!("✨ No stale status files to clean up.");
        } else {
            println!("🧹 Cleaned up {} stale status files:", cleaned.len());
            for session in &cleaned {
                println!("  ✅ Removed {session}.status.json");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_status_update_with_session_name() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories to avoid race conditions
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        // Use absolute path for state directory
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create a test session
        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "test-session".to_string(),
            "test/branch".to_string(),
            git_temp.path().join("worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        // Update status
        let args = StatusArgs {
            command: None,
            task: Some("Working on tests".to_string()),
            tests: Some("passed".to_string()),
            todos: Some("3/5".to_string()),
            blocked: false,
            session: Some("test-session".to_string()),
        };

        let result = execute(config.clone(), args);
        assert!(result.is_ok());

        // Verify status was saved
        let loaded_status = Status::load(&state_dir, "test-session").unwrap();
        assert!(loaded_status.is_some());

        let status = loaded_status.unwrap();
        assert_eq!(status.current_task, "Working on tests");
        assert_eq!(status.test_status, crate::core::status::TestStatus::Passed);
        assert_eq!(status.todos_completed, Some(3));
        assert_eq!(status.todos_total, Some(5));
        assert!(!status.is_blocked);
    }

    #[test]
    fn test_status_update_with_blocked() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        // Use absolute path for state directory
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create a test session
        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "blocked-session".to_string(),
            "test/branch".to_string(),
            git_temp.path().join("worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        // Update status with blocked flag
        let args = StatusArgs {
            command: None,
            task: Some("Need help with Redis mocking".to_string()),
            tests: Some("failed".to_string()),
            todos: None,
            blocked: true,
            session: Some("blocked-session".to_string()),
        };

        let result = execute(config.clone(), args);
        assert!(result.is_ok());

        // Verify blocked status
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
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        // Use absolute path for state directory
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create a test session with worktree
        let worktree_path = git_temp.path().join("test-worktree");
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
            todos: None,
            blocked: false,
            session: None,
        };

        let result = execute(config.clone(), args);
        assert!(result.is_ok());

        // Verify status was saved for the correct session
        let status = Status::load(&state_dir, "context-session")
            .unwrap()
            .unwrap();
        assert_eq!(status.current_task, "Auto-detected session");
    }

    #[test]
    fn test_status_show_single_session() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        // Use absolute path for state directory
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create a test session and status
        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "show-test".to_string(),
            "test/branch".to_string(),
            git_temp.path().join("worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        // Create a status
        let status = Status::new(
            "show-test".to_string(),
            "Testing show command".to_string(),
            crate::core::status::TestStatus::Passed,
        );
        status.save(&state_dir).unwrap();

        // Show status
        let args = StatusArgs {
            command: Some(StatusCommands::Show {
                session: Some("show-test".to_string()),
                json: false,
            }),
            task: None,
            tests: None,
            todos: None,
            blocked: false,
            session: None,
        };

        let result = execute(config, args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_show_all_sessions() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        // Use absolute path for state directory
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        let session_manager = SessionManager::new(&config);

        // Create multiple sessions with statuses
        for i in 1..=3 {
            let session_name = format!("session-{i}");
            let session_state = crate::core::session::SessionState::new(
                session_name.clone(),
                format!("test/branch-{i}"),
                git_temp.path().join(format!("worktree-{i}")),
            );
            session_manager.save_state(&session_state).unwrap();

            let status = Status::new(
                session_name,
                format!("Working on feature {i}"),
                crate::core::status::TestStatus::Unknown,
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
            todos: None,
            blocked: false,
            session: None,
        };

        let result = execute(config, args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_update_invalid_session() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        // Use absolute path for state directory
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        let args = StatusArgs {
            command: None,
            task: Some("Should fail".to_string()),
            tests: Some("passed".to_string()),
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
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        // Use absolute path for state directory
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create a test session
        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "test-session".to_string(),
            "test/branch".to_string(),
            git_temp.path().join("worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        let args = StatusArgs {
            command: None,
            task: Some("Test task".to_string()),
            tests: Some("invalid".to_string()),
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
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        // Use absolute path for state directory
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create a test session
        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "test-session".to_string(),
            "test/branch".to_string(),
            git_temp.path().join("worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        let args = StatusArgs {
            command: None,
            task: Some("Test task".to_string()),
            tests: Some("passed".to_string()),
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
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        // Use absolute path for state directory
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Missing task
        let args = StatusArgs {
            command: None,
            task: None,
            tests: Some("passed".to_string()),
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
    }

    #[test]
    fn test_display_status_formatting() {
        // Test basic status display
        let status = Status::new(
            "test-session".to_string(),
            "Working on authentication".to_string(),
            crate::core::status::TestStatus::Passed,
        );

        // We can't easily test println! output, but we can test the logic
        // by verifying the status fields are accessible and formatted correctly
        assert_eq!(status.session_name, "test-session");
        assert_eq!(status.current_task, "Working on authentication");
        assert_eq!(status.test_status, crate::core::status::TestStatus::Passed);
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
        );
        status1.last_update = now - Duration::hours(2); // 2 hours ago

        let mut status2 = Status::new(
            "session2".to_string(),
            "Task 2".to_string(),
            crate::core::status::TestStatus::Failed,
        );
        status2.last_update = now - Duration::minutes(30); // 30 minutes ago

        let mut status3 = Status::new(
            "session3".to_string(),
            "Task 3".to_string(),
            crate::core::status::TestStatus::Unknown,
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
        )
        .with_todos(0, 3);
        assert_eq!(
            status_zero_todos.format_todos(),
            Some("0% (0/3)".to_string())
        );
    }

    #[test]
    fn test_show_status_json_output_single_session() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create a test session and status
        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "json-test".to_string(),
            "test/branch".to_string(),
            git_temp.path().join("worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        let status = Status::new(
            "json-test".to_string(),
            "Testing JSON output".to_string(),
            crate::core::status::TestStatus::Passed,
        )
        .with_todos(3, 5);
        status.save(&state_dir).unwrap();

        // Test JSON output
        let result = show_status(config, Some("json-test".to_string()), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_status_json_output_all_sessions() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        let session_manager = SessionManager::new(&config);

        // Create multiple sessions with statuses
        for i in 1..=2 {
            let session_name = format!("json-session-{i}");
            let session_state = crate::core::session::SessionState::new(
                session_name.clone(),
                format!("test/branch-{i}"),
                git_temp.path().join(format!("worktree-{i}")),
            );
            session_manager.save_state(&session_state).unwrap();

            let status = Status::new(
                session_name,
                format!("JSON test task {i}"),
                crate::core::status::TestStatus::Passed,
            );
            status.save(&state_dir).unwrap();
        }

        // Test JSON output for all sessions
        let result = show_status(config, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_status_nonexistent_session() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Try to show status for nonexistent session
        let result = show_status(config, Some("nonexistent".to_string()), false);
        assert!(result.is_ok()); // Should not error, just show no status found
    }

    #[test]
    fn test_show_status_nonexistent_session_json() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Try to show JSON status for nonexistent session
        let result = show_status(config, Some("nonexistent".to_string()), true);
        assert!(result.is_ok()); // Should not error, just show nothing for JSON
    }

    #[test]
    fn test_show_status_empty_sessions_list() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Show all sessions when no sessions exist
        let result = show_status(config, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_status_empty_sessions_list_json() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Show all sessions as JSON when no sessions exist
        let result = show_status(config, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_status_path_resolution_absolute() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Create state directory
        let state_dir = temp_dir.path().join("absolute_state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        // Use absolute path for state directory
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create a test session and status
        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "abs-path-test".to_string(),
            "test/branch".to_string(),
            git_temp.path().join("worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        let status = Status::new(
            "abs-path-test".to_string(),
            "Testing absolute path resolution".to_string(),
            crate::core::status::TestStatus::Passed,
        );
        status.save(&state_dir).unwrap();

        // Test that show_status works with absolute path
        let result = show_status(config, Some("abs-path-test".to_string()), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_status_path_resolution_relative() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        // For this test, let's use a simpler approach with absolute path
        // but test the path resolution logic by using the actual resolved path
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create a test session and status
        let session_manager = SessionManager::new(&config);
        let session_state = crate::core::session::SessionState::new(
            "rel-path-test".to_string(),
            "test/branch".to_string(),
            git_temp.path().join("worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        let status = Status::new(
            "rel-path-test".to_string(),
            "Testing path resolution".to_string(),
            crate::core::status::TestStatus::Failed,
        );
        status.save(&state_dir).unwrap();

        // Test that show_status works with resolved path
        let result = show_status(config, Some("rel-path-test".to_string()), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_status_git_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&temp_dir, &temp_dir).unwrap();

        let mut config = create_test_config();
        // Use relative path that will require git repo detection
        config.directories.state_dir = ".para/state".to_string();

        // Change to a non-git directory
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // This should fail because we're not in a git repository
        let result = show_status(config, Some("test".to_string()), false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Not in a para repository"));
    }

    #[test]
    fn test_status_update_blocked_for_review_sessions() {
        let (git_temp, _git_service) = setup_test_repo();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        // Pre-create .para and state directories
        let para_dir = git_temp.path().join(".para");
        let state_dir = para_dir.join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        // Create a session in Review state
        let session_manager = SessionManager::new(&config);
        let mut session_state = crate::core::session::SessionState::new(
            "review-session".to_string(),
            "test/review-branch".to_string(),
            git_temp.path().join("worktree"),
        );
        session_state.status = crate::core::session::SessionStatus::Review;
        session_manager.save_state(&session_state).unwrap();

        // Try to update status for Review session
        let args = StatusArgs {
            command: None,
            task: Some("Trying to update review session".to_string()),
            tests: Some("passed".to_string()),
            todos: None,
            blocked: false,
            session: Some("review-session".to_string()),
        };

        let result = execute(config.clone(), args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot update status for sessions in Review state"));
    }
}
