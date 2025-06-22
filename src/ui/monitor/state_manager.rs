use crate::ui::monitor::service::SessionService;
use crate::ui::monitor::state::MonitorAppState;
use crate::ui::monitor::SessionInfo;

/// Context information about the current state
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StateContext {
    pub selected_index: usize,
    pub session_count: usize,
    pub show_stale: bool,
    pub mode: crate::ui::monitor::AppMode,
}

/// Manages application state and session data
pub struct StateManager {
    service: SessionService,
}

#[allow(dead_code)]
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

    /// Handle selection change in the given direction
    pub fn handle_selection_change(
        &self,
        state: &mut MonitorAppState,
        direction: SelectionDirection,
        sessions: &[SessionInfo],
    ) {
        match direction {
            SelectionDirection::Next => state.next_item(sessions),
            SelectionDirection::Previous => state.previous_item(sessions),
        }
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

    /// Transition the application mode
    pub fn transition_mode(
        &self,
        state: &mut MonitorAppState,
        new_mode: crate::ui::monitor::AppMode,
    ) {
        state.mode = new_mode;
    }

    /// Toggle the stale sessions visibility
    pub fn toggle_stale_visibility(&self, state: &mut MonitorAppState) {
        state.toggle_stale();
    }

    /// Get current state context for other components
    pub fn get_current_context(
        &self,
        state: &MonitorAppState,
        sessions: &[SessionInfo],
    ) -> StateContext {
        StateContext {
            selected_index: state.selected_index,
            session_count: sessions.len(),
            show_stale: state.show_stale,
            mode: state.mode,
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

    /// Get the currently selected session if any
    pub fn get_selected_session<'a>(
        &self,
        state: &MonitorAppState,
        sessions: &'a [SessionInfo],
    ) -> Option<&'a SessionInfo> {
        state.get_selected_session(sessions)
    }

    /// Validate state consistency and fix any issues
    pub fn validate_and_fix_state(&self, state: &mut MonitorAppState, sessions: &[SessionInfo]) {
        // Ensure selection is within bounds
        if state.selected_index >= sessions.len() && !sessions.is_empty() {
            state.selected_index = sessions.len() - 1;
        }

        // Ensure table state is consistent with selected index
        if sessions.is_empty() {
            state.table_state.select(None);
        } else {
            state.table_state.select(Some(state.selected_index));
        }
    }

    /// Reset state to initial values
    pub fn reset_state(&self, state: &mut MonitorAppState) {
        state.selected_index = 0;
        state.mode = crate::ui::monitor::AppMode::Normal;
        state.should_quit = false;
        state.table_state.select(Some(0));
        // Clear input by taking it (which empties it)
        state.take_input();
        state.clear_error();
        state.clear_expired_feedback();
    }

    /// Initialize state for a fresh session list
    pub fn initialize_state(&self, state: &mut MonitorAppState, sessions: &[SessionInfo]) {
        if sessions.is_empty() {
            state.selected_index = 0;
            state.table_state.select(None);
        } else {
            state.selected_index = 0;
            state.table_state.select(Some(0));
        }
    }
}

/// Direction for selection changes
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum SelectionDirection {
    Next,
    Previous,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::ui::monitor::{AppMode, SessionStatus};

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
                confidence: None,
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
                confidence: None,
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
                confidence: None,
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
        let state_manager = StateManager::new(service);

        // Basic creation test
        let state = MonitorAppState::new();
        let sessions = create_test_sessions();
        let context = state_manager.get_current_context(&state, &sessions);

        assert_eq!(context.selected_index, 0);
        assert_eq!(context.session_count, 3);
        assert_eq!(context.mode, AppMode::Normal);
    }

    #[test]
    fn test_selection_handling() {
        let config = create_test_config();
        let service = SessionService::new(config);
        let state_manager = StateManager::new(service);
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test next selection
        assert_eq!(state.selected_index, 0);
        state_manager.handle_selection_change(&mut state, SelectionDirection::Next, &sessions);
        assert_eq!(state.selected_index, 1);

        state_manager.handle_selection_change(&mut state, SelectionDirection::Next, &sessions);
        assert_eq!(state.selected_index, 2);

        // Test boundary - shouldn't go beyond last item
        state_manager.handle_selection_change(&mut state, SelectionDirection::Next, &sessions);
        assert_eq!(state.selected_index, 2);

        // Test previous selection
        state_manager.handle_selection_change(&mut state, SelectionDirection::Previous, &sessions);
        assert_eq!(state.selected_index, 1);

        state_manager.handle_selection_change(&mut state, SelectionDirection::Previous, &sessions);
        assert_eq!(state.selected_index, 0);

        // Test boundary - shouldn't go below 0
        state_manager.handle_selection_change(&mut state, SelectionDirection::Previous, &sessions);
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
    fn test_state_validation_and_fixing() {
        let config = create_test_config();
        let service = SessionService::new(config);
        let state_manager = StateManager::new(service);
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test fixing out of bounds selection
        state.selected_index = 999;
        state_manager.validate_and_fix_state(&mut state, &sessions);
        assert_eq!(state.selected_index, 2); // Should be clamped to last valid index
        assert_eq!(state.table_state.selected(), Some(2));

        // Test with empty sessions
        let empty_sessions = vec![];
        state.selected_index = 5;
        state_manager.validate_and_fix_state(&mut state, &empty_sessions);
        assert_eq!(state.selected_index, 5); // Should not change for empty sessions
        assert_eq!(state.table_state.selected(), None);
    }

    #[test]
    fn test_state_initialization() {
        let config = create_test_config();
        let service = SessionService::new(config);
        let state_manager = StateManager::new(service);
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test initialization with sessions
        state_manager.initialize_state(&mut state, &sessions);
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.table_state.selected(), Some(0));

        // Test initialization with empty sessions
        let empty_sessions = vec![];
        state_manager.initialize_state(&mut state, &empty_sessions);
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.table_state.selected(), None);
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
    fn test_mode_transitions() {
        let config = create_test_config();
        let service = SessionService::new(config);
        let state_manager = StateManager::new(service);
        let mut state = MonitorAppState::new();

        // Test mode transitions
        assert_eq!(state.mode, AppMode::Normal);

        state_manager.transition_mode(&mut state, AppMode::FinishPrompt);
        assert_eq!(state.mode, AppMode::FinishPrompt);

        state_manager.transition_mode(&mut state, AppMode::CancelConfirm);
        assert_eq!(state.mode, AppMode::CancelConfirm);

        state_manager.transition_mode(&mut state, AppMode::ErrorDialog);
        assert_eq!(state.mode, AppMode::ErrorDialog);

        state_manager.transition_mode(&mut state, AppMode::Normal);
        assert_eq!(state.mode, AppMode::Normal);
    }

    #[test]
    fn test_stale_visibility_toggle() {
        let config = create_test_config();
        let service = SessionService::new(config);
        let state_manager = StateManager::new(service);
        let mut state = MonitorAppState::new();

        // Test stale toggle
        let initial_stale = state.show_stale;
        state_manager.toggle_stale_visibility(&mut state);
        assert_eq!(state.show_stale, !initial_stale);

        state_manager.toggle_stale_visibility(&mut state);
        assert_eq!(state.show_stale, initial_stale);
    }

    #[test]
    fn test_state_context() {
        let config = create_test_config();
        let service = SessionService::new(config);
        let state_manager = StateManager::new(service);
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test context retrieval
        state.selected_index = 1;
        state.mode = AppMode::FinishPrompt;
        state.show_stale = false;

        let context = state_manager.get_current_context(&state, &sessions);
        assert_eq!(context.selected_index, 1);
        assert_eq!(context.session_count, 3);
        assert!(!context.show_stale);
        assert_eq!(context.mode, AppMode::FinishPrompt);
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

    #[test]
    fn test_state_reset() {
        let config = create_test_config();
        let service = SessionService::new(config);
        let state_manager = StateManager::new(service);
        let mut state = MonitorAppState::new();

        // Modify state
        state.selected_index = 5;
        state.mode = AppMode::FinishPrompt;
        state.quit();
        state.add_char('t');
        state.show_error("Test error".to_string());

        // Reset state
        state_manager.reset_state(&mut state);

        // Verify reset
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.mode, AppMode::Normal);
        assert!(!state.should_quit);
        assert_eq!(state.table_state.selected(), Some(0));
        assert_eq!(state.get_input(), "");
        assert!(state.error_message.is_none());
    }

    #[test]
    fn test_selected_session_retrieval() {
        let config = create_test_config();
        let service = SessionService::new(config);
        let state_manager = StateManager::new(service);
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test getting selected session
        state.selected_index = 1;
        let selected = state_manager.get_selected_session(&state, &sessions);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().name, "session2");

        // Test with out of bounds index
        state.selected_index = 999;
        let selected = state_manager.get_selected_session(&state, &sessions);
        assert!(selected.is_none());

        // Test with empty sessions
        let empty_sessions = vec![];
        state.selected_index = 0;
        let selected = state_manager.get_selected_session(&state, &empty_sessions);
        assert!(selected.is_none());
    }
}
