use crate::cli::parser::{StatusArgs, StatusCommands};
use crate::config::Config;
use crate::core::session::SessionManager;
use crate::core::status::Status;
use crate::utils::{get_main_repository_root, ParaError, Result};
use serde_json;
use std::path::{Path, PathBuf};

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

    // Save status to file in the main repository's state directory
    let state_dir = StatePathResolver::resolve_state_dir(&config)?;

    status
        .save(&state_dir)
        .map_err(|e| ParaError::config_error(e.to_string()))?;

    println!("Status updated for session '{}'", session_name);

    Ok(())
}

fn show_status(config: Config, session: Option<String>, json: bool) -> Result<()> {
    let state_dir = StatePathResolver::resolve_state_dir(&config)?;
    let session_manager = SessionManager::new(&config);
    let fetcher = StatusFetcher::new(&config, &session_manager, &state_dir);

    let formatter: Box<dyn StatusFormatter> = if json {
        Box::new(JsonFormatter)
    } else {
        Box::new(TextFormatter)
    };

    match session {
        Some(session_name) => {
            let status = fetcher.fetch_single_status(&session_name)?;
            match status {
                Some(s) => {
                    let output = formatter.format_single(&s)?;
                    println!("{}", output);
                }
                None => {
                    if !json {
                        println!("No status found for session '{}'", session_name);
                    }
                }
            }
        }
        None => {
            let statuses = fetcher.fetch_all_statuses()?;
            let output = formatter.format_multiple(&statuses)?;
            println!("{}", output);
        }
    }

    Ok(())
}

// Status fetcher module
pub struct StatusFetcher<'a> {
    config: &'a Config,
    session_manager: &'a SessionManager,
    state_dir: &'a PathBuf,
}

impl<'a> StatusFetcher<'a> {
    pub fn new(
        config: &'a Config,
        session_manager: &'a SessionManager,
        state_dir: &'a PathBuf,
    ) -> Self {
        Self {
            config,
            session_manager,
            state_dir,
        }
    }

    pub fn fetch_single_status(&self, session_name: &str) -> Result<Option<Status>> {
        Status::load(self.state_dir, session_name)
            .map_err(|e| ParaError::config_error(e.to_string()))
    }

    pub fn fetch_all_statuses(&self) -> Result<Vec<Status>> {
        let sessions = self.session_manager.list_sessions()?;
        let mut statuses = Vec::new();

        for session_state in sessions {
            if let Some(status) = Status::load(self.state_dir, &session_state.name)
                .map_err(|e| ParaError::config_error(e.to_string()))?
            {
                statuses.push(status);
            }
        }

        Ok(statuses)
    }
}

// Status formatter module
pub trait StatusFormatter {
    fn format_single(&self, status: &Status) -> Result<String>;
    fn format_multiple(&self, statuses: &[Status]) -> Result<String>;
}

pub struct JsonFormatter;

impl StatusFormatter for JsonFormatter {
    fn format_single(&self, status: &Status) -> Result<String> {
        serde_json::to_string_pretty(status)
            .map_err(|e| ParaError::config_error(format!("Failed to serialize status: {}", e)))
    }

    fn format_multiple(&self, statuses: &[Status]) -> Result<String> {
        serde_json::to_string_pretty(statuses)
            .map_err(|e| ParaError::config_error(format!("Failed to serialize status: {}", e)))
    }
}

pub struct TextFormatter;

impl StatusFormatter for TextFormatter {
    fn format_single(&self, status: &Status) -> Result<String> {
        let mut output = String::new();
        output.push_str(&format!("Session: {}\n", status.session_name));
        output.push_str(&format!("Task: {}\n", status.current_task));
        output.push_str(&format!("Tests: {}\n", status.test_status));
        output.push_str(&format!("Confidence: {}\n", status.confidence));

        if let Some(todos) = status.format_todos() {
            output.push_str(&format!("Progress: {}\n", todos));
        }

        if status.is_blocked {
            output.push_str("Status: BLOCKED\n");
            if let Some(reason) = &status.blocked_reason {
                output.push_str(&format!("Reason: {}\n", reason));
            }
        }

        output.push_str(&format!(
            "Last Update: {}",
            status.last_update.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        Ok(output)
    }

    fn format_multiple(&self, statuses: &[Status]) -> Result<String> {
        if statuses.is_empty() {
            return Ok("No session statuses found.".to_string());
        }

        // Sort by last update time (most recent first)
        let mut sorted_statuses = statuses.to_vec();
        sorted_statuses.sort_by(|a, b| b.last_update.cmp(&a.last_update));

        let mut output = String::new();
        output.push_str(&format!(
            "{:<20} {:<40} {:<10} {:<10} {:<15} {:<10}\n",
            "Session", "Current Task", "Tests", "Confidence", "Progress", "Status"
        ));
        output.push_str(&format!("{}\n", "-".repeat(110)));

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

            output.push_str(&format!(
                "{:<20} {:<40} {:<10} {:<10} {:<15} {:<10}\n",
                status.session_name,
                task,
                status.test_status.to_string(),
                status.confidence.to_string(),
                progress,
                status_str
            ));
        }

        Ok(output)
    }
}

// State path resolver module
pub struct StatePathResolver;

impl StatePathResolver {
    pub fn resolve_state_dir(config: &Config) -> Result<PathBuf> {
        if Path::new(&config.directories.state_dir).is_absolute() {
            // If state_dir is already absolute (e.g., in tests), use it directly
            Ok(PathBuf::from(&config.directories.state_dir))
        } else {
            // Otherwise, resolve it relative to the main repo root
            let repo_root = get_main_repository_root()
                .map_err(|e| ParaError::git_error(format!("Not in a para repository: {}", e)))?;
            Ok(repo_root.join(&config.directories.state_dir))
        }
    }
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
            confidence: Some("high".to_string()),
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
            confidence: Some("low".to_string()),
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
}