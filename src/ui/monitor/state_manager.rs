use crate::ui::monitor::service::SessionService;
use crate::ui::monitor::state::MonitorAppState;
use crate::ui::monitor::SessionInfo;

/// Manages application state and session data
pub struct StateManager {
    service: SessionService,
}

impl StateManager {
    pub fn new(service: SessionService) -> Self {
        Self { service }
    }

    /// Load sessions from the service based on current state
    pub fn load_sessions(&self, show_stale: bool) -> Vec<SessionInfo> {
        self.service
            .load_sessions(show_stale)
            .unwrap_or_else(|_| Vec::new())
    }

    /// Update the sessions list and adjust state accordingly
    pub fn update_sessions(
        &self,
        state: &mut MonitorAppState,
        sessions: Vec<SessionInfo>,
    ) -> Vec<SessionInfo> {
        state.update_selection_for_sessions(&sessions);
        sessions
    }

    /// Handle selection change to a specific index (from mouse click)
    pub fn handle_selection_to_index(
        &self,
        state: &mut MonitorAppState,
        index: usize,
        sessions: &[SessionInfo],
    ) {
        if index < sessions.len() {
            state.selected_index = index;
            state.table_state.select(Some(index));
        }
    }

    /// Check if the application should quit
    pub fn should_quit(&self, state: &MonitorAppState) -> bool {
        state.should_quit
    }

    /// Check if the application should refresh
    pub fn should_refresh(&self, state: &MonitorAppState) -> bool {
        state.should_refresh()
    }

    /// Mark the application as refreshed
    pub fn mark_refreshed(&self, state: &mut MonitorAppState) {
        state.mark_refreshed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::ui::monitor::SessionStatus;

    fn create_test_config() -> Config {
        crate::test_utils::test_helpers::create_test_config()
    }

    fn create_test_sessions() -> Vec<SessionInfo> {
        vec![
            SessionInfo {
                name: "session1".to_string(),
                branch: "branch1".to_string(),
                status: SessionStatus::Active,
                last_activity: chrono::Utc::now(),
                task: "Task 1".to_string(),
                worktree_path: std::path::PathBuf::from("/tmp/session1"),
                test_status: None,
                diff_stats: None,
                todo_percentage: None,
                is_blocked: false,
            },
            SessionInfo {
                name: "session2".to_string(),
                branch: "branch2".to_string(),
                status: SessionStatus::Idle,
                last_activity: chrono::Utc::now(),
                task: "Task 2".to_string(),
                worktree_path: std::path::PathBuf::from("/tmp/session2"),
                test_status: None,
                diff_stats: None,
                todo_percentage: None,
                is_blocked: false,
            },
            SessionInfo {
                name: "session3".to_string(),
                branch: "branch3".to_string(),
                status: SessionStatus::Ready,
                last_activity: chrono::Utc::now(),
                task: "Task 3".to_string(),
                worktree_path: std::path::PathBuf::from("/tmp/session3"),
                test_status: None,
                diff_stats: None,
                todo_percentage: None,
                is_blocked: false,
            },
        ]
    }

    #[test]
    fn test_state_manager_creation() {
        let config = create_test_config();
        let service = SessionService::new(config);
        let _state_manager = StateManager::new(service);

        // Basic creation test - just verify no panic
        let state = MonitorAppState::new();
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_selection_to_index() {
        let config = create_test_config();
        let service = SessionService::new(config);
        let state_manager = StateManager::new(service);
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test valid index selection
        state_manager.handle_selection_to_index(&mut state, 2, &sessions);
        assert_eq!(state.selected_index, 2);
        assert_eq!(state.table_state.selected(), Some(2));

        // Test invalid index (out of bounds)
        let original_index = state.selected_index;
        state_manager.handle_selection_to_index(&mut state, 999, &sessions);
        assert_eq!(state.selected_index, original_index); // Should not change
    }

    #[test]
    fn test_session_updates() {
        let config = create_test_config();
        let service = SessionService::new(config);
        let state_manager = StateManager::new(service);
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test updating sessions
        let _updated_sessions = state_manager.update_sessions(&mut state, sessions.clone());
        assert_eq!(_updated_sessions.len(), 3);
        assert_eq!(state.table_state.selected(), Some(0));

        // Test with different selection
        state.selected_index = 2;
        let _updated_sessions2 = state_manager.update_sessions(&mut state, sessions);
        assert_eq!(state.selected_index, 2);
        assert_eq!(state.table_state.selected(), Some(2));
    }

    #[test]
    fn test_refresh_and_quit_checks() {
        let config = create_test_config();
        let service = SessionService::new(config);
        let state_manager = StateManager::new(service);
        let mut state = MonitorAppState::new();

        // Test quit check
        assert!(!state_manager.should_quit(&state));
        state.quit();
        assert!(state_manager.should_quit(&state));

        // Test refresh check
        let mut fresh_state = MonitorAppState::new();
        assert!(!state_manager.should_refresh(&fresh_state));

        // Mark as refreshed
        state_manager.mark_refreshed(&mut fresh_state);
        assert!(!state_manager.should_refresh(&fresh_state));
    }
}
