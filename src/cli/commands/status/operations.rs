use crate::core::session::SessionManager;
use crate::core::status::Status;
use crate::utils::{ParaError, Result};
use std::path::Path;

pub struct StatusOperations<'a> {
    session_manager: &'a SessionManager,
}

impl<'a> StatusOperations<'a> {
    pub fn new(session_manager: &'a SessionManager) -> Self {
        Self { session_manager }
    }

    pub fn update_status(
        &self,
        state_dir: &Path,
        session: &str,
        update: StatusUpdate,
    ) -> Result<()> {
        // Verify session exists
        if !self.session_manager.session_exists(session) {
            return Err(ParaError::session_not_found(session));
        }

        // Parse and validate arguments
        let test_status = Status::parse_test_status(&update.tests)
            .map_err(|e| ParaError::invalid_args(e.to_string()))?;
        let confidence_level = Status::parse_confidence(&update.confidence)
            .map_err(|e| ParaError::invalid_args(e.to_string()))?;

        // Create status object
        let mut status = Status::new(
            session.to_string(),
            update.task,
            test_status,
            confidence_level,
        );

        // Handle optional todos
        if let Some(todos_str) = update.todos {
            let (completed, total) = Status::parse_todos(&todos_str)
                .map_err(|e| ParaError::invalid_args(e.to_string()))?;
            status = status.with_todos(completed, total);
        }

        // Handle blocked state
        if update.blocked {
            // If blocked, use the task description as the blocked reason
            let task_description = status.current_task.clone();
            status = status.with_blocked(Some(task_description));
        }

        // Save status to file
        status
            .save(state_dir)
            .map_err(|e| ParaError::config_error(e.to_string()))?;

        println!("Status updated for session '{}'", session);
        Ok(())
    }

    pub fn get_status(&self, state_dir: &Path, session: &str) -> Result<Option<Status>> {
        Status::load(state_dir, session).map_err(|e| ParaError::config_error(e.to_string()))
    }

    pub fn get_all_statuses(&self, state_dir: &Path) -> Result<Vec<Status>> {
        let sessions = self.session_manager.list_sessions()?;
        let mut statuses = Vec::new();

        for session_state in sessions {
            if let Some(status) = Status::load(state_dir, &session_state.name)
                .map_err(|e| ParaError::config_error(e.to_string()))?
            {
                statuses.push(status);
            }
        }

        Ok(statuses)
    }
}

#[derive(Debug)]
pub struct StatusUpdate {
    pub task: String,
    pub tests: String,
    pub confidence: String,
    pub todos: Option<String>,
    pub blocked: bool,
}