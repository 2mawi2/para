use crate::ui::monitor::actions::MonitorActions;
use crate::ui::monitor::event_handler::{
    DialogAction, NavigationAction, SessionAction, SystemAction, UiAction,
};
use crate::ui::monitor::state::{ButtonClick, MonitorAppState};
use crate::ui::monitor::SessionInfo;
use crate::utils::Result;
use copypasta::{ClipboardContext, ClipboardProvider};

/// Dispatches and executes UI actions
pub struct ActionDispatcher {
    actions: MonitorActions,
}

impl ActionDispatcher {
    pub fn new(actions: MonitorActions) -> Self {
        Self { actions }
    }

    /// Dispatch a UI action and execute it
    pub fn dispatch(
        &mut self,
        action: UiAction,
        state: &mut MonitorAppState,
        sessions: &[SessionInfo],
    ) -> Result<ActionResult> {
        match action {
            UiAction::Session(session_action) => {
                self.execute_session_action(session_action, state, sessions)
            }
            UiAction::Navigation(nav_action) => {
                self.execute_navigation_action(nav_action, state, sessions);
                Ok(ActionResult::Continue)
            }
            UiAction::Dialog(dialog_action) => {
                self.execute_dialog_action(dialog_action, state, sessions)
            }
            UiAction::System(system_action) => {
                self.execute_system_action(system_action, state);
                Ok(ActionResult::Continue)
            }
        }
    }

    fn execute_session_action(
        &mut self,
        action: SessionAction,
        state: &mut MonitorAppState,
        sessions: &[SessionInfo],
    ) -> Result<ActionResult> {
        match action {
            SessionAction::Resume(index) => {
                if let Some(session) = sessions.get(index) {
                    // Register button click for visual feedback
                    state.register_button_click(ButtonClick::Resume(index));

                    if let Err(e) = self.actions.resume_session(session) {
                        state.show_error(format!("Failed to resume session: {}", e));
                    } else {
                        state.show_feedback(format!("Opening session: {}", session.name));
                    }
                }
                Ok(ActionResult::Continue)
            }
            SessionAction::Copy(index) => {
                if let Some(session) = sessions.get(index) {
                    // Register button click for visual feedback
                    state.register_button_click(ButtonClick::Copy(index));

                    match ClipboardContext::new() {
                        Ok(mut ctx) => {
                            if let Err(e) = ctx.set_contents(session.name.clone()) {
                                state.show_error(format!("Failed to copy to clipboard: {}", e));
                            } else {
                                state.show_feedback(format!("Copied: {}", session.name));
                            }
                        }
                        Err(e) => {
                            state.show_error(format!("Clipboard not available: {}", e));
                        }
                    }
                }
                Ok(ActionResult::Continue)
            }
            SessionAction::Integrate(index) => {
                if let Some(session) = sessions.get(index) {
                    self.actions.integrate_session(session)?;
                }
                Ok(ActionResult::Continue)
            }
            SessionAction::Finish(index) => {
                if let Some(_session) = sessions.get(index) {
                    // Register button click for visual feedback
                    state.register_button_click(ButtonClick::Finish(index));
                    // Start the finish dialog instead of directly finishing
                    state.start_finish();
                }
                Ok(ActionResult::Continue)
            }
            SessionAction::Cancel(index) => {
                if let Some(_session) = sessions.get(index) {
                    // Register button click for visual feedback
                    state.register_button_click(ButtonClick::Cancel(index));
                    // Start the cancel confirmation dialog instead of directly canceling
                    state.start_cancel();
                }
                Ok(ActionResult::Continue)
            }
        }
    }

    fn execute_navigation_action(
        &self,
        action: NavigationAction,
        state: &mut MonitorAppState,
        sessions: &[SessionInfo],
    ) {
        match action {
            NavigationAction::SelectNext => {
                state.next_item(sessions);
            }
            NavigationAction::SelectPrevious => {
                state.previous_item(sessions);
            }
            NavigationAction::ToggleStale => {
                state.toggle_stale();
            }
        }
    }

    fn execute_dialog_action(
        &mut self,
        action: DialogAction,
        state: &mut MonitorAppState,
        sessions: &[SessionInfo],
    ) -> Result<ActionResult> {
        match action {
            DialogAction::StartFinish => {
                state.start_finish();
                Ok(ActionResult::Continue)
            }
            DialogAction::StartCancel => {
                state.start_cancel();
                Ok(ActionResult::Continue)
            }
            DialogAction::ExitDialog => {
                state.exit_dialog();
                Ok(ActionResult::Continue)
            }
            DialogAction::AddChar(c) => {
                state.add_char(c);
                Ok(ActionResult::Continue)
            }
            DialogAction::Backspace => {
                state.backspace();
                Ok(ActionResult::Continue)
            }
            DialogAction::ExecuteFinish => {
                if let Some(session) = state.get_selected_session(sessions) {
                    let message = state.take_input();
                    self.actions.finish_session(session, message)?;
                    state.exit_dialog();
                    Ok(ActionResult::RefreshSessions)
                } else {
                    Ok(ActionResult::Continue)
                }
            }
            DialogAction::ExecuteCancel => {
                if let Some(session) = state.get_selected_session(sessions) {
                    self.actions.cancel_session(session)?;
                    state.exit_dialog();
                    Ok(ActionResult::RefreshSessions)
                } else {
                    state.exit_dialog();
                    Ok(ActionResult::Continue)
                }
            }
            DialogAction::ClearError => {
                state.clear_error();
                Ok(ActionResult::Continue)
            }
        }
    }

    fn execute_system_action(&self, action: SystemAction, state: &mut MonitorAppState) {
        match action {
            SystemAction::Quit => {
                state.quit();
            }
        }
    }
}

/// Result of executing an action
#[derive(Debug, Clone, PartialEq)]
pub enum ActionResult {
    Continue,
    RefreshSessions,
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
        ]
    }

    #[test]
    fn test_navigation_actions() {
        let config = create_test_config();
        let actions = MonitorActions::new(config);
        let dispatcher = ActionDispatcher::new(actions);
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test select next
        assert_eq!(state.selected_index, 0);
        dispatcher.execute_navigation_action(NavigationAction::SelectNext, &mut state, &sessions);
        assert_eq!(state.selected_index, 1);

        // Test select previous
        dispatcher.execute_navigation_action(
            NavigationAction::SelectPrevious,
            &mut state,
            &sessions,
        );
        assert_eq!(state.selected_index, 0);

        // Test toggle stale
        let initial_stale = state.show_stale;
        dispatcher.execute_navigation_action(NavigationAction::ToggleStale, &mut state, &sessions);
        assert_eq!(state.show_stale, !initial_stale);
    }

    #[test]
    fn test_dialog_actions() {
        let config = create_test_config();
        let actions = MonitorActions::new(config);
        let mut dispatcher = ActionDispatcher::new(actions);
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test start finish
        let result = dispatcher
            .execute_dialog_action(DialogAction::StartFinish, &mut state, &sessions)
            .unwrap();
        assert_eq!(result, ActionResult::Continue);
        assert_eq!(state.mode, crate::ui::monitor::AppMode::FinishPrompt);

        // Test add char
        dispatcher
            .execute_dialog_action(DialogAction::AddChar('t'), &mut state, &sessions)
            .unwrap();
        assert_eq!(state.get_input(), "t");

        // Test backspace
        dispatcher
            .execute_dialog_action(DialogAction::Backspace, &mut state, &sessions)
            .unwrap();
        assert_eq!(state.get_input(), "");

        // Test exit dialog
        dispatcher
            .execute_dialog_action(DialogAction::ExitDialog, &mut state, &sessions)
            .unwrap();
        assert_eq!(state.mode, crate::ui::monitor::AppMode::Normal);
    }

    #[test]
    fn test_system_actions() {
        let config = create_test_config();
        let actions = MonitorActions::new(config);
        let dispatcher = ActionDispatcher::new(actions);
        let mut state = MonitorAppState::new();

        // Test quit
        assert!(!state.should_quit);
        dispatcher.execute_system_action(SystemAction::Quit, &mut state);
        assert!(state.should_quit);
    }

    #[test]
    fn test_session_copy_action() {
        let config = create_test_config();
        let actions = MonitorActions::new(config);
        let mut dispatcher = ActionDispatcher::new(actions);
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test copy action
        let result = dispatcher
            .execute_session_action(SessionAction::Copy(0), &mut state, &sessions)
            .unwrap();
        assert_eq!(result, ActionResult::Continue);

        // Verify button click was registered
        assert!(state.get_button_click().is_some());
        if let Some(click) = state.get_button_click() {
            assert_eq!(*click, ButtonClick::Copy(0));
        }
    }

    #[test]
    fn test_dispatch_integration() {
        let config = create_test_config();
        let actions = MonitorActions::new(config);
        let mut dispatcher = ActionDispatcher::new(actions);
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test dispatching a navigation action
        let result = dispatcher
            .dispatch(
                UiAction::Navigation(NavigationAction::SelectNext),
                &mut state,
                &sessions,
            )
            .unwrap();
        assert_eq!(result, ActionResult::Continue);
        assert_eq!(state.selected_index, 1);

        // Test dispatching a system action
        let result = dispatcher
            .dispatch(UiAction::System(SystemAction::Quit), &mut state, &sessions)
            .unwrap();
        assert_eq!(result, ActionResult::Continue);
        assert!(state.should_quit);

        // Test dispatching a dialog action
        let mut state = MonitorAppState::new(); // Reset state
        let result = dispatcher
            .dispatch(
                UiAction::Dialog(DialogAction::StartFinish),
                &mut state,
                &sessions,
            )
            .unwrap();
        assert_eq!(result, ActionResult::Continue);
        assert_eq!(state.mode, crate::ui::monitor::AppMode::FinishPrompt);
    }

    #[test]
    fn test_invalid_session_indices() {
        let config = create_test_config();
        let actions = MonitorActions::new(config);
        let mut dispatcher = ActionDispatcher::new(actions);
        let mut state = MonitorAppState::new();
        let sessions = create_test_sessions();

        // Test copy with invalid index
        let result = dispatcher
            .execute_session_action(SessionAction::Copy(999), &mut state, &sessions)
            .unwrap();
        assert_eq!(result, ActionResult::Continue);

        // No button click should be registered for invalid index
        assert!(state.get_button_click().is_none());

        // Test resume with invalid index
        let result = dispatcher
            .execute_session_action(SessionAction::Resume(999), &mut state, &sessions)
            .unwrap();
        assert_eq!(result, ActionResult::Continue);
    }
}
