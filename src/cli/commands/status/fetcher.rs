use crate::config::Config;
use crate::core::session::SessionManager;
use crate::core::status::Status;
use crate::utils::{ParaError, Result};
use std::path::PathBuf;

pub struct StatusFetcher<'a> {
    config: &'a Config,
    session_manager: &'a SessionManager,
    state_dir: &'a PathBuf,
}

impl<'a> StatusFetcher<'a> {
    pub fn new(config: &'a Config, session_manager: &'a SessionManager, state_dir: &'a PathBuf) -> Self {
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