/// Controller layer for MVC pattern implementation
/// This module handles user interactions and coordinates between model and view layers
use crate::ui::monitor::data::{SessionData, SessionDataStatus};
use crate::ui::monitor::presentation::{PresentationTheme, SessionViewModel};
use crate::ui::monitor::state::MonitorAppState;
use crate::ui::monitor::{AppMode, SessionInfo};
use crate::utils::{ParaError, Result};
use std::collections::HashMap;

/// Controller actions that can be performed on sessions
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum SessionAction {
    SelectSession(usize),
    RefreshSessions,
    ToggleStaleFilter,
    StartFinish(String),
    CancelSession(String),
    ResumeSession(String),
    DispatchAgent {
        session_name: String,
        prompt: String,
    },
    ShowError(String),
    ClearError,
}

/// Controller events that can be emitted
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum SessionEvent {
    SessionsUpdated(Vec<SessionData>),
    SelectionChanged(usize),
    ModeChanged(AppMode),
    ErrorOccurred(String),
    ActionCompleted(SessionAction),
}

/// Controller that manages session interactions following MVC pattern
#[allow(dead_code)]
pub struct SessionController {
    /// Current session data models
    models: Vec<SessionData>,
    /// View models with presentation logic
    view_models: Vec<SessionViewModel>,
    /// Presentation theme
    theme: PresentationTheme,
    /// Session name to model mapping for fast lookup
    session_lookup: HashMap<String, usize>,
}

#[allow(dead_code)]
impl SessionController {
    /// Create a new session controller
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            view_models: Vec::new(),
            theme: PresentationTheme::default(),
            session_lookup: HashMap::new(),
        }
    }

    /// Create controller with custom theme
    pub fn with_theme(theme: PresentationTheme) -> Self {
        Self {
            models: Vec::new(),
            view_models: Vec::new(),
            theme,
            session_lookup: HashMap::new(),
        }
    }

    /// Handle a user action and return resulting events
    pub fn handle_action(
        &mut self,
        action: SessionAction,
        state: &mut MonitorAppState,
    ) -> Result<Vec<SessionEvent>> {
        let mut events = Vec::new();

        match &action {
            SessionAction::SelectSession(index) => {
                if *index < self.models.len() {
                    state.selected_index = *index;
                    state.table_state.select(Some(*index));
                    events.push(SessionEvent::SelectionChanged(*index));
                }
            }

            SessionAction::RefreshSessions => {
                // This would normally trigger a service refresh
                // For now, we just emit an event that the coordinator can handle
                events.push(SessionEvent::ActionCompleted(action));
            }

            SessionAction::ToggleStaleFilter => {
                state.toggle_stale();
                events.push(SessionEvent::ActionCompleted(action));
            }

            SessionAction::StartFinish(session_name) => {
                if self.session_lookup.contains_key(session_name) {
                    state.start_finish();
                    events.push(SessionEvent::ModeChanged(AppMode::FinishPrompt));
                    events.push(SessionEvent::ActionCompleted(action));
                } else {
                    events.push(SessionEvent::ErrorOccurred(format!(
                        "Session '{}' not found",
                        session_name
                    )));
                }
            }

            SessionAction::CancelSession(session_name) => {
                if self.session_lookup.contains_key(session_name) {
                    state.start_cancel();
                    events.push(SessionEvent::ModeChanged(AppMode::CancelConfirm));
                    events.push(SessionEvent::ActionCompleted(action));
                } else {
                    events.push(SessionEvent::ErrorOccurred(format!(
                        "Session '{}' not found",
                        session_name
                    )));
                }
            }

            SessionAction::ResumeSession(session_name) => {
                if let Some(&index) = self.session_lookup.get(session_name) {
                    state.selected_index = index;
                    state.table_state.select(Some(index));
                    events.push(SessionEvent::SelectionChanged(index));
                    events.push(SessionEvent::ActionCompleted(action));
                } else {
                    events.push(SessionEvent::ErrorOccurred(format!(
                        "Session '{}' not found",
                        session_name
                    )));
                }
            }

            SessionAction::DispatchAgent {
                session_name,
                prompt: _,
            } => {
                if self.session_lookup.contains_key(session_name) {
                    // This would normally trigger agent dispatch
                    events.push(SessionEvent::ActionCompleted(action));
                } else {
                    events.push(SessionEvent::ErrorOccurred(format!(
                        "Session '{}' not found",
                        session_name
                    )));
                }
            }

            SessionAction::ShowError(message) => {
                state.show_error(message.clone());
                events.push(SessionEvent::ErrorOccurred(message.clone()));
                events.push(SessionEvent::ModeChanged(AppMode::ErrorDialog));
            }

            SessionAction::ClearError => {
                state.clear_error();
                events.push(SessionEvent::ModeChanged(AppMode::Normal));
            }
        }

        Ok(events)
    }

    /// Update the models and recreate view models
    pub fn update_models(&mut self, session_infos: Vec<SessionInfo>) -> Vec<SessionEvent> {
        // Convert SessionInfo to SessionData models
        self.models = session_infos.into_iter().map(|info| info.into()).collect();

        // Rebuild lookup table
        self.session_lookup.clear();
        for (index, model) in self.models.iter().enumerate() {
            self.session_lookup.insert(model.name.clone(), index);
        }

        // Create view models from data models
        self.view_models = self
            .models
            .iter()
            .map(|model| SessionViewModel::with_theme(model.clone(), self.theme.clone()))
            .collect();

        vec![SessionEvent::SessionsUpdated(self.models.clone())]
    }

    /// Get the current data models
    #[allow(dead_code)]
    pub fn get_models(&self) -> &[SessionData] {
        &self.models
    }

    /// Get the current view models
    #[allow(dead_code)]
    pub fn get_view_models(&self) -> &[SessionViewModel] {
        &self.view_models
    }

    /// Get view models enhanced with UI state
    pub fn get_enhanced_view_models(&self, state: &MonitorAppState) -> Vec<SessionViewModel> {
        self.view_models
            .iter()
            .enumerate()
            .map(|(index, view_model)| {
                let is_selected = index == state.selected_index;
                view_model.clone().with_selection(is_selected)
            })
            .collect()
    }

    /// Get the selected session data
    #[allow(dead_code)]
    pub fn get_selected_session(&self, state: &MonitorAppState) -> Option<&SessionData> {
        self.models.get(state.selected_index)
    }

    /// Get the selected view model
    #[allow(dead_code)]
    pub fn get_selected_view_model(&self, state: &MonitorAppState) -> Option<SessionViewModel> {
        self.view_models
            .get(state.selected_index)
            .map(|vm| vm.clone().with_selection(true))
    }

    /// Find session by name
    pub fn find_session_by_name(&self, name: &str) -> Option<(usize, &SessionData)> {
        self.session_lookup
            .get(name)
            .and_then(|&index| self.models.get(index).map(|model| (index, model)))
    }

    /// Get sessions that need immediate attention
    pub fn get_sessions_needing_attention(&self) -> Vec<(usize, &SessionData)> {
        self.models
            .iter()
            .enumerate()
            .filter(|(_, model)| {
                model.needs_immediate_attention() || model.status.needs_attention()
            })
            .collect()
    }

    /// Get session statistics for display
    pub fn get_session_statistics(&self) -> SessionStatistics {
        let mut stats = SessionStatistics::default();

        for model in &self.models {
            match model.status {
                SessionDataStatus::Active => stats.active_count += 1,
                SessionDataStatus::Idle => stats.idle_count += 1,
                SessionDataStatus::Review => stats.review_count += 1,
                SessionDataStatus::Ready => stats.ready_count += 1,
                SessionDataStatus::Stale => stats.stale_count += 1,
            }

            if model.is_blocked {
                stats.blocked_count += 1;
            }

            if model.has_test_failures() {
                stats.failed_tests_count += 1;
            }

            if model.is_complete() {
                stats.completed_count += 1;
            }
        }

        stats.total_count = self.models.len();
        stats
    }

    /// Validate an action before execution
    pub fn validate_action(&self, action: &SessionAction) -> Result<()> {
        match action {
            SessionAction::SelectSession(index) => {
                if *index >= self.models.len() {
                    return Err(ParaError::invalid_args(format!(
                        "Invalid session index: {}",
                        index
                    )));
                }
            }

            SessionAction::StartFinish(session_name)
            | SessionAction::CancelSession(session_name)
            | SessionAction::ResumeSession(session_name) => {
                if !self.session_lookup.contains_key(session_name) {
                    return Err(ParaError::session_not_found(session_name.clone()));
                }
            }

            SessionAction::DispatchAgent {
                session_name,
                prompt,
            } => {
                if !self.session_lookup.contains_key(session_name) {
                    return Err(ParaError::session_not_found(session_name.clone()));
                }
                if prompt.trim().is_empty() {
                    return Err(ParaError::invalid_args("Dispatch prompt cannot be empty"));
                }
            }

            // These actions are always valid
            SessionAction::RefreshSessions
            | SessionAction::ToggleStaleFilter
            | SessionAction::ShowError(_)
            | SessionAction::ClearError => {}
        }

        Ok(())
    }
}

impl Default for SessionController {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the current session collection
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionStatistics {
    pub total_count: usize,
    pub active_count: usize,
    pub idle_count: usize,
    pub review_count: usize,
    pub ready_count: usize,
    pub stale_count: usize,
    pub blocked_count: usize,
    pub failed_tests_count: usize,
    pub completed_count: usize,
}

#[allow(dead_code)]
impl SessionStatistics {
    /// Get a summary string of the statistics
    pub fn summary(&self) -> String {
        format!(
            "Total: {} | Active: {} | Review: {} | Blocked: {} | Failed: {}",
            self.total_count,
            self.active_count,
            self.review_count,
            self.blocked_count,
            self.failed_tests_count
        )
    }

    /// Get the percentage of sessions that need attention
    pub fn attention_percentage(&self) -> f32 {
        if self.total_count == 0 {
            0.0
        } else {
            (self.review_count + self.ready_count + self.blocked_count + self.failed_tests_count)
                as f32
                / self.total_count as f32
                * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::status::TestStatus;
    use chrono::Utc;
    use std::path::PathBuf;

    #[allow(dead_code)]
    fn create_test_session_data(name: &str, status: SessionDataStatus) -> SessionData {
        SessionData::new(
            name.to_string(),
            format!("branch-{}", name),
            status,
            Utc::now(),
            format!("Task for {}", name),
            PathBuf::from(format!("/test/{}", name)),
        )
    }

    fn create_test_session_info(
        name: &str,
        status: crate::ui::monitor::SessionStatus,
    ) -> SessionInfo {
        SessionInfo {
            name: name.to_string(),
            branch: format!("branch-{}", name),
            status,
            last_activity: Utc::now(),
            task: format!("Task for {}", name),
            worktree_path: PathBuf::from(format!("/test/{}", name)),
            test_status: None,
            confidence: None,
            todo_percentage: None,
            is_blocked: false,
        }
    }

    #[test]
    fn test_controller_creation() {
        let controller = SessionController::new();
        assert_eq!(controller.models.len(), 0);
        assert_eq!(controller.view_models.len(), 0);
        assert!(controller.session_lookup.is_empty());
    }

    #[test]
    fn test_controller_with_theme() {
        let theme = PresentationTheme::default();
        let controller = SessionController::with_theme(theme.clone());
        assert_eq!(controller.theme.active_color, theme.active_color);
    }

    #[test]
    fn test_update_models() {
        let mut controller = SessionController::new();
        let session_infos = vec![
            create_test_session_info("session1", crate::ui::monitor::SessionStatus::Active),
            create_test_session_info("session2", crate::ui::monitor::SessionStatus::Review),
        ];

        let events = controller.update_models(session_infos);

        assert_eq!(controller.models.len(), 2);
        assert_eq!(controller.view_models.len(), 2);
        assert_eq!(controller.session_lookup.len(), 2);
        assert!(controller.session_lookup.contains_key("session1"));
        assert!(controller.session_lookup.contains_key("session2"));
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SessionEvent::SessionsUpdated(_)));
    }

    #[test]
    fn test_handle_select_session() {
        let mut controller = SessionController::new();
        let mut state = MonitorAppState::new();

        // Add some test data
        let session_infos = vec![
            create_test_session_info("session1", crate::ui::monitor::SessionStatus::Active),
            create_test_session_info("session2", crate::ui::monitor::SessionStatus::Review),
        ];
        controller.update_models(session_infos);

        let action = SessionAction::SelectSession(1);
        let events = controller.handle_action(action, &mut state).unwrap();

        assert_eq!(state.selected_index, 1);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SessionEvent::SelectionChanged(1)));
    }

    #[test]
    fn test_handle_select_invalid_session() {
        let mut controller = SessionController::new();
        let mut state = MonitorAppState::new();

        let action = SessionAction::SelectSession(5); // Invalid index
        let events = controller.handle_action(action, &mut state).unwrap();

        assert_eq!(state.selected_index, 0); // Should remain unchanged
        assert_eq!(events.len(), 0); // No events for invalid selection
    }

    #[test]
    fn test_handle_start_finish() {
        let mut controller = SessionController::new();
        let mut state = MonitorAppState::new();

        // Add test session
        let session_infos = vec![create_test_session_info(
            "test-session",
            crate::ui::monitor::SessionStatus::Active,
        )];
        controller.update_models(session_infos);

        let action = SessionAction::StartFinish("test-session".to_string());
        let events = controller
            .handle_action(action.clone(), &mut state)
            .unwrap();

        assert_eq!(state.mode, AppMode::FinishPrompt);
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            SessionEvent::ModeChanged(AppMode::FinishPrompt)
        ));
        assert!(matches!(events[1], SessionEvent::ActionCompleted(_)));
    }

    #[test]
    fn test_handle_start_finish_nonexistent_session() {
        let mut controller = SessionController::new();
        let mut state = MonitorAppState::new();

        let action = SessionAction::StartFinish("nonexistent".to_string());
        let events = controller.handle_action(action, &mut state).unwrap();

        assert_eq!(state.mode, AppMode::Normal); // Should remain unchanged
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SessionEvent::ErrorOccurred(_)));
    }

    #[test]
    fn test_handle_toggle_stale_filter() {
        let mut controller = SessionController::new();
        let mut state = MonitorAppState::new();
        let original_show_stale = state.show_stale;

        let action = SessionAction::ToggleStaleFilter;
        let events = controller
            .handle_action(action.clone(), &mut state)
            .unwrap();

        assert_eq!(state.show_stale, !original_show_stale);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SessionEvent::ActionCompleted(_)));
    }

    #[test]
    fn test_handle_error_actions() {
        let mut controller = SessionController::new();
        let mut state = MonitorAppState::new();

        // Test show error
        let action = SessionAction::ShowError("Test error".to_string());
        let events = controller.handle_action(action, &mut state).unwrap();

        assert_eq!(state.mode, AppMode::ErrorDialog);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], SessionEvent::ErrorOccurred(_)));
        assert!(matches!(
            events[1],
            SessionEvent::ModeChanged(AppMode::ErrorDialog)
        ));

        // Test clear error
        let action = SessionAction::ClearError;
        let events = controller.handle_action(action, &mut state).unwrap();

        assert_eq!(state.mode, AppMode::Normal);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            SessionEvent::ModeChanged(AppMode::Normal)
        ));
    }

    #[test]
    fn test_get_enhanced_view_models() {
        let mut controller = SessionController::new();
        let mut state = MonitorAppState::new();
        state.selected_index = 1;

        let session_infos = vec![
            create_test_session_info("session1", crate::ui::monitor::SessionStatus::Active),
            create_test_session_info("session2", crate::ui::monitor::SessionStatus::Review),
            create_test_session_info("session3", crate::ui::monitor::SessionStatus::Idle),
        ];
        controller.update_models(session_infos);

        let enhanced_view_models = controller.get_enhanced_view_models(&state);

        assert_eq!(enhanced_view_models.len(), 3);
        assert!(!enhanced_view_models[0].is_selected); // First session not selected
        assert!(enhanced_view_models[1].is_selected); // Second session selected
        assert!(!enhanced_view_models[2].is_selected); // Third session not selected
    }

    #[test]
    fn test_find_session_by_name() {
        let mut controller = SessionController::new();
        let session_infos = vec![
            create_test_session_info("session1", crate::ui::monitor::SessionStatus::Active),
            create_test_session_info("session2", crate::ui::monitor::SessionStatus::Review),
        ];
        controller.update_models(session_infos);

        let result = controller.find_session_by_name("session2");
        assert!(result.is_some());
        let (index, model) = result.unwrap();
        assert_eq!(index, 1);
        assert_eq!(model.name, "session2");

        let result = controller.find_session_by_name("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_sessions_needing_attention() {
        let mut controller = SessionController::new();

        // Create sessions with different attention needs
        let mut session_infos = vec![
            create_test_session_info("normal", crate::ui::monitor::SessionStatus::Active),
            create_test_session_info("review", crate::ui::monitor::SessionStatus::Review),
            create_test_session_info("blocked", crate::ui::monitor::SessionStatus::Active),
        ];

        // Make one session blocked
        session_infos[2].is_blocked = true;
        session_infos[2].test_status = Some(TestStatus::Failed);

        controller.update_models(session_infos);

        let needing_attention = controller.get_sessions_needing_attention();

        // Should include review session and blocked session
        assert_eq!(needing_attention.len(), 2);
        assert_eq!(needing_attention[0].1.name, "review");
        assert_eq!(needing_attention[1].1.name, "blocked");
    }

    #[test]
    fn test_session_statistics() {
        let mut controller = SessionController::new();

        let mut session_infos = vec![
            create_test_session_info("active1", crate::ui::monitor::SessionStatus::Active),
            create_test_session_info("active2", crate::ui::monitor::SessionStatus::Active),
            create_test_session_info("review1", crate::ui::monitor::SessionStatus::Review),
            create_test_session_info("idle1", crate::ui::monitor::SessionStatus::Idle),
            create_test_session_info("stale1", crate::ui::monitor::SessionStatus::Stale),
        ];

        // Add some test status and completion
        session_infos[0].test_status = Some(TestStatus::Failed);
        session_infos[1].is_blocked = true;
        session_infos[2].todo_percentage = Some(100);

        controller.update_models(session_infos);

        let stats = controller.get_session_statistics();

        assert_eq!(stats.total_count, 5);
        assert_eq!(stats.active_count, 2);
        assert_eq!(stats.review_count, 1);
        assert_eq!(stats.idle_count, 1);
        assert_eq!(stats.stale_count, 1);
        assert_eq!(stats.blocked_count, 1);
        assert_eq!(stats.failed_tests_count, 1);
        assert_eq!(stats.completed_count, 1);

        // Test summary and percentage
        let summary = stats.summary();
        assert!(summary.contains("Total: 5"));
        assert!(summary.contains("Active: 2"));

        let attention_pct = stats.attention_percentage();
        assert!(attention_pct > 0.0); // Should have some sessions needing attention
    }

    #[test]
    fn test_validate_action() {
        let mut controller = SessionController::new();
        let session_infos = vec![create_test_session_info(
            "session1",
            crate::ui::monitor::SessionStatus::Active,
        )];
        controller.update_models(session_infos);

        // Valid actions
        assert!(controller
            .validate_action(&SessionAction::SelectSession(0))
            .is_ok());
        assert!(controller
            .validate_action(&SessionAction::StartFinish("session1".to_string()))
            .is_ok());
        assert!(controller
            .validate_action(&SessionAction::RefreshSessions)
            .is_ok());
        assert!(controller
            .validate_action(&SessionAction::DispatchAgent {
                session_name: "session1".to_string(),
                prompt: "test prompt".to_string()
            })
            .is_ok());

        // Invalid actions
        assert!(controller
            .validate_action(&SessionAction::SelectSession(5))
            .is_err());
        assert!(controller
            .validate_action(&SessionAction::StartFinish("nonexistent".to_string()))
            .is_err());
        assert!(controller
            .validate_action(&SessionAction::DispatchAgent {
                session_name: "session1".to_string(),
                prompt: "   ".to_string() // Empty prompt
            })
            .is_err());
    }
}
