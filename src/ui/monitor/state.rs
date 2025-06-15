use crate::ui::monitor::{AppMode, SessionInfo};
use ratatui::widgets::TableState;
use std::time::Instant;

/// UI state management for the monitor interface
pub struct MonitorAppState {
    pub selected_index: usize,
    pub should_quit: bool,
    pub table_state: TableState,
    pub mode: AppMode,
    pub input_buffer: String,
    pub show_stale: bool,
    pub last_refresh: Instant,
}

impl MonitorAppState {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        Self {
            selected_index: 0,
            should_quit: false,
            table_state,
            mode: AppMode::Normal,
            input_buffer: String::new(),
            show_stale: false,
            last_refresh: Instant::now(),
        }
    }

    /// Navigate to the previous item in the session list
    pub fn previous_item(&mut self, _sessions: &[SessionInfo]) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.table_state.select(Some(self.selected_index));
        }
    }

    /// Navigate to the next item in the session list
    pub fn next_item(&mut self, sessions: &[SessionInfo]) {
        if self.selected_index < sessions.len().saturating_sub(1) {
            self.selected_index += 1;
            self.table_state.select(Some(self.selected_index));
        }
    }

    /// Update the selected index when sessions change (e.g., after refresh)
    pub fn update_selection_for_sessions(&mut self, sessions: &[SessionInfo]) {
        if self.selected_index >= sessions.len() && !sessions.is_empty() {
            self.selected_index = sessions.len() - 1;
            self.table_state.select(Some(self.selected_index));
        } else if sessions.is_empty() {
            self.selected_index = 0;
            self.table_state.select(None);
        } else {
            // Ensure table state is in sync
            self.table_state.select(Some(self.selected_index));
        }
    }

    /// Get the currently selected session
    pub fn get_selected_session<'a>(&self, sessions: &'a [SessionInfo]) -> Option<&'a SessionInfo> {
        sessions.get(self.selected_index)
    }

    /// Start the finish prompt mode
    pub fn start_finish(&mut self) {
        self.mode = AppMode::FinishPrompt;
        self.input_buffer.clear();
    }

    /// Start the cancel confirmation mode
    pub fn start_cancel(&mut self) {
        self.mode = AppMode::CancelConfirm;
    }

    /// Exit any dialog mode and return to normal
    pub fn exit_dialog(&mut self) {
        self.mode = AppMode::Normal;
        self.input_buffer.clear();
    }

    /// Check if it's time to refresh (every 2 seconds)
    pub fn should_refresh(&self) -> bool {
        self.last_refresh.elapsed().as_secs() >= 2
    }

    /// Mark that a refresh just occurred
    pub fn mark_refreshed(&mut self) {
        self.last_refresh = Instant::now();
    }

    /// Toggle showing stale sessions
    pub fn toggle_stale(&mut self) {
        self.show_stale = !self.show_stale;
    }

    /// Add a character to the input buffer (for commit messages)
    pub fn add_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    /// Remove the last character from input buffer
    pub fn backspace(&mut self) {
        self.input_buffer.pop();
    }

    /// Get the current input buffer content
    pub fn get_input(&self) -> &str {
        &self.input_buffer
    }

    /// Check if input is ready for submission (not empty when trimmed)
    pub fn is_input_ready(&self) -> bool {
        !self.input_buffer.trim().is_empty()
    }

    /// Consume the current input buffer and return its content
    pub fn take_input(&mut self) -> String {
        let input = self.input_buffer.clone();
        self.input_buffer.clear();
        input
    }

    /// Request to quit the application
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

impl Default for MonitorAppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::monitor::SessionStatus;
    use chrono::Utc;
    use std::path::PathBuf;

    fn create_test_sessions() -> Vec<SessionInfo> {
        vec![
            SessionInfo {
                name: "session1".to_string(),
                branch: "branch1".to_string(),
                status: SessionStatus::Active,
                last_activity: Utc::now(),
                task: "Task 1".to_string(),
                worktree_path: PathBuf::from("/tmp/session1"),
            },
            SessionInfo {
                name: "session2".to_string(),
                branch: "branch2".to_string(),
                status: SessionStatus::Idle,
                last_activity: Utc::now(),
                task: "Task 2".to_string(),
                worktree_path: PathBuf::from("/tmp/session2"),
            },
            SessionInfo {
                name: "session3".to_string(),
                branch: "branch3".to_string(),
                status: SessionStatus::Ready,
                last_activity: Utc::now(),
                task: "Task 3".to_string(),
                worktree_path: PathBuf::from("/tmp/session3"),
            },
        ]
    }

    #[test]
    fn test_state_creation() {
        let state = MonitorAppState::new();
        assert_eq!(state.selected_index, 0);
        assert!(!state.should_quit);
        assert_eq!(state.mode, AppMode::Normal);
        assert!(state.input_buffer.is_empty());
        assert!(!state.show_stale);
    }

    #[test]
    fn test_navigation() {
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test next navigation
        assert_eq!(state.selected_index, 0);
        state.next_item(&sessions);
        assert_eq!(state.selected_index, 1);
        state.next_item(&sessions);
        assert_eq!(state.selected_index, 2);

        // Test boundary (shouldn't go beyond last item)
        state.next_item(&sessions);
        assert_eq!(state.selected_index, 2);

        // Test previous navigation
        state.previous_item(&sessions);
        assert_eq!(state.selected_index, 1);
        state.previous_item(&sessions);
        assert_eq!(state.selected_index, 0);

        // Test boundary (shouldn't go below 0)
        state.previous_item(&sessions);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_selection_update() {
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Set selection beyond bounds
        state.selected_index = 5;
        state.update_selection_for_sessions(&sessions);
        assert_eq!(state.selected_index, 2); // Should clamp to last valid index

        // Test with empty sessions
        state.update_selection_for_sessions(&[]);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_get_selected_session() {
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test valid selection
        state.selected_index = 1;
        let selected = state.get_selected_session(&sessions);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().name, "session2");

        // Test invalid selection
        state.selected_index = 10;
        let selected = state.get_selected_session(&sessions);
        assert!(selected.is_none());
    }

    #[test]
    fn test_mode_transitions() {
        let mut state = MonitorAppState::new();

        // Test starting finish mode
        assert_eq!(state.mode, AppMode::Normal);
        state.start_finish();
        assert_eq!(state.mode, AppMode::FinishPrompt);
        assert!(state.input_buffer.is_empty());

        // Test starting cancel mode
        state.start_cancel();
        assert_eq!(state.mode, AppMode::CancelConfirm);

        // Test exiting dialog
        state.exit_dialog();
        assert_eq!(state.mode, AppMode::Normal);
    }

    #[test]
    fn test_input_handling() {
        let mut state = MonitorAppState::new();

        // Test adding characters
        assert!(state.get_input().is_empty());
        state.add_char('h');
        state.add_char('i');
        assert_eq!(state.get_input(), "hi");

        // Test backspace
        state.backspace();
        assert_eq!(state.get_input(), "h");

        // Test input readiness
        assert!(state.is_input_ready());

        // Reset for taking input test
        let _ = state.take_input(); // Clear current input
        assert!(!state.is_input_ready()); // Empty input is not ready

        // Test taking input (which clears)
        state.add_char('t');
        state.add_char('e');
        state.add_char('s');
        state.add_char('t');
        let input = state.take_input();
        assert_eq!(input, "test");
        assert!(state.get_input().is_empty());
        assert!(!state.is_input_ready()); // Empty input is not ready after taking
    }

    #[test]
    fn test_refresh_timing() {
        let mut state = MonitorAppState::new();

        // Should not need refresh immediately
        assert!(!state.should_refresh());

        // Mark as refreshed and check again
        state.mark_refreshed();
        assert!(!state.should_refresh());
    }

    #[test]
    fn test_toggles() {
        let mut state = MonitorAppState::new();

        // Test stale toggle
        assert!(!state.show_stale);
        state.toggle_stale();
        assert!(state.show_stale);
        state.toggle_stale();
        assert!(!state.show_stale);

        // Test quit
        assert!(!state.should_quit);
        state.quit();
        assert!(state.should_quit);
    }
}
