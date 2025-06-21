use crate::config::Config;
use crate::ui::monitor::actions::MonitorActions;
use crate::ui::monitor::renderer::MonitorRenderer;
use crate::ui::monitor::service::SessionService;
use crate::ui::monitor::state::MonitorAppState;
use crate::ui::monitor::{AppMode, SessionInfo};
use crate::utils::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;

/// High-level coordinator for the monitor UI that manages all components
pub struct MonitorCoordinator {
    pub state: MonitorAppState,
    pub sessions: Vec<SessionInfo>,
    renderer: MonitorRenderer,
    actions: MonitorActions,
    service: SessionService,
}

impl MonitorCoordinator {
    pub fn new(config: Config) -> Self {
        let renderer = MonitorRenderer::new(config.clone());
        let actions = MonitorActions::new(config.clone());
        let service = SessionService::new(config.clone());
        let state = MonitorAppState::new();

        let mut coordinator = Self {
            state,
            sessions: Vec::new(),
            renderer,
            actions,
            service,
        };

        coordinator.refresh_sessions();
        coordinator
    }

    pub fn refresh_sessions(&mut self) {
        self.sessions = self
            .service
            .load_sessions(self.state.show_stale)
            .unwrap_or_else(|_| Vec::new());
        self.state.update_selection_for_sessions(&self.sessions);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.state.mode {
            AppMode::Normal => self.handle_normal_key(key),
            AppMode::FinishPrompt => self.handle_finish_prompt_key(key),
            AppMode::CancelConfirm => self.handle_cancel_confirm_key(key),
            AppMode::ErrorDialog => self.handle_error_dialog_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.state.quit(),
            KeyCode::Char('c') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.state.quit();
                } else {
                    self.start_cancel();
                }
            }
            KeyCode::Char('s') => {
                self.state.toggle_stale();
                self.refresh_sessions();
            }
            KeyCode::Up | KeyCode::Char('k') => self.state.previous_item(&self.sessions),
            KeyCode::Down | KeyCode::Char('j') => self.state.next_item(&self.sessions),
            KeyCode::Enter => self.resume_selected()?,
            KeyCode::Char('f') => self.start_finish(),
            KeyCode::Char('i') => self.integrate_if_ready()?,
            _ => {}
        }
        Ok(())
    }

    fn handle_finish_prompt_key(&mut self, key: KeyEvent) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Esc => {
                self.state.exit_dialog();
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.exit_dialog();
            }
            KeyCode::Enter => {
                if self.state.is_input_ready() {
                    self.execute_finish()?;
                }
            }
            KeyCode::Backspace => {
                self.state.backspace();
            }
            KeyCode::Char(c) => {
                self.state.add_char(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_cancel_confirm_key(&mut self, key: KeyEvent) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Enter => {
                self.execute_cancel()?;
                self.state.exit_dialog();
            }
            KeyCode::Esc => {
                self.state.exit_dialog();
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.exit_dialog();
            }
            _ => {}
        }
        Ok(())
    }

    fn resume_selected(&mut self) -> Result<()> {
        if let Some(session) = self.state.get_selected_session(&self.sessions) {
            if let Err(e) = self.actions.resume_session(session) {
                self.state
                    .show_error(format!("Failed to resume session: {}", e));
            }
        }
        Ok(())
    }

    fn start_finish(&mut self) {
        if self.state.get_selected_session(&self.sessions).is_some() {
            self.state.start_finish();
        }
    }

    fn start_cancel(&mut self) {
        if self.state.get_selected_session(&self.sessions).is_some() {
            self.state.start_cancel();
        }
    }

    fn integrate_if_ready(&mut self) -> Result<()> {
        if let Some(session) = self.state.get_selected_session(&self.sessions) {
            self.actions.integrate_session(session)?;
        }
        Ok(())
    }

    fn execute_finish(&mut self) -> Result<()> {
        if let Some(session) = self.state.get_selected_session(&self.sessions) {
            let message = self.state.take_input();
            self.actions.finish_session(session, message)?;
            self.state.exit_dialog();
            self.refresh_sessions();
        }
        Ok(())
    }

    fn execute_cancel(&mut self) -> Result<()> {
        if let Some(session) = self.state.get_selected_session(&self.sessions) {
            self.actions.cancel_session(session)?;
            self.refresh_sessions();
        }
        Ok(())
    }

    fn handle_error_dialog_key(&mut self, key: KeyEvent) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
                self.state.clear_error();
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.clear_error();
            }
            _ => {}
        }
        Ok(())
    }

    pub fn should_quit(&self) -> bool {
        self.state.should_quit
    }

    pub fn should_refresh(&self) -> bool {
        self.state.should_refresh()
    }

    pub fn mark_refreshed(&mut self) {
        self.state.mark_refreshed();
    }

    pub fn render(&self, f: &mut Frame) {
        self.renderer.render(f, &self.sessions, &self.state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent};

    fn create_test_config() -> Config {
        crate::test_utils::test_helpers::create_test_config()
    }

    #[test]
    fn test_coordinator_creation() {
        let config = create_test_config();
        let coordinator = MonitorCoordinator::new(config);

        assert_eq!(coordinator.state.mode, AppMode::Normal);
        assert!(!coordinator.should_quit());
        assert_eq!(coordinator.sessions.len(), 0); // No sessions in test environment
    }

    #[test]
    fn test_coordinator_navigation() {
        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Test basic state
        assert_eq!(coordinator.state.selected_index, 0);
        assert!(coordinator.state.show_stale); // Now defaults to true

        // Test stale toggle
        coordinator.state.toggle_stale();
        assert!(!coordinator.state.show_stale);
    }

    #[test]
    fn test_coordinator_mode_transitions() {
        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Test finish mode
        coordinator.start_finish();
        assert_eq!(coordinator.state.mode, AppMode::Normal); // No sessions to finish

        // Test cancel mode
        coordinator.start_cancel();
        assert_eq!(coordinator.state.mode, AppMode::Normal); // No sessions to cancel
    }

    #[test]
    fn test_error_dialog_key_handling() {
        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Set up error state
        coordinator.state.show_error("Test error".to_string());
        assert_eq!(coordinator.state.mode, AppMode::ErrorDialog);

        // Test Enter key dismisses error
        let enter_key = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::NONE);
        coordinator.handle_key(enter_key).unwrap();
        assert_eq!(coordinator.state.mode, AppMode::Normal);
        assert!(coordinator.state.error_message.is_none());

        // Test Esc key dismisses error
        coordinator.state.show_error("Test error 2".to_string());
        let esc_key = KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE);
        coordinator.handle_key(esc_key).unwrap();
        assert_eq!(coordinator.state.mode, AppMode::Normal);

        // Test Space key dismisses error
        coordinator.state.show_error("Test error 3".to_string());
        let space_key = KeyEvent::new(KeyCode::Char(' '), crossterm::event::KeyModifiers::NONE);
        coordinator.handle_key(space_key).unwrap();
        assert_eq!(coordinator.state.mode, AppMode::Normal);
    }

    #[test]
    fn test_coordinator_refresh_timing() {
        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Should not need refresh immediately
        assert!(!coordinator.should_refresh());

        // Mark as refreshed
        coordinator.mark_refreshed();
        assert!(!coordinator.should_refresh());
    }

    #[test]
    fn test_handle_key_normal_mode() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Test quit key
        let quit_key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(quit_key).is_ok());
        assert!(coordinator.should_quit());

        // Test navigation keys
        let mut coordinator = MonitorCoordinator::new(create_test_config());
        let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert!(coordinator.handle_key(up_key).is_ok());

        let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert!(coordinator.handle_key(down_key).is_ok());

        // Test stale toggle (now starts as true)
        assert!(coordinator.state.show_stale);
        let stale_key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(stale_key).is_ok());
        assert!(!coordinator.state.show_stale);
    }

    #[test]
    fn test_handle_key_finish_prompt_mode() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);
        coordinator.state.start_finish();

        // Test typing in finish prompt
        let char_key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(char_key).is_ok());
        assert_eq!(coordinator.state.get_input(), "t");

        // Test escape
        let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert!(coordinator.handle_key(esc_key).is_ok());
        assert_eq!(coordinator.state.mode, AppMode::Normal);
    }

    #[test]
    fn test_handle_key_cancel_confirm_mode() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);
        coordinator.state.start_cancel();

        // Test escape
        let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert!(coordinator.handle_key(esc_key).is_ok());
        assert_eq!(coordinator.state.mode, AppMode::Normal);
    }

    // COMPREHENSIVE INTEGRATION TESTS - PHASE 1
    // These tests verify complete workflows and cross-component communication

    #[test]
    fn test_integration_complete_session_loading_workflow() {
        use crate::test_utils::test_helpers::*;
        use tempfile::TempDir;

        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let config = create_test_config_with_dir(&temp_dir);
        let mut coordinator = MonitorCoordinator::new(config);

        // Initial state verification
        assert_eq!(coordinator.sessions.len(), 0);
        assert_eq!(coordinator.state.mode, AppMode::Normal);
        assert!(!coordinator.should_quit());

        // Test session refresh workflow
        coordinator.refresh_sessions();

        // Session loading should complete without errors
        assert_eq!(coordinator.state.mode, AppMode::Normal);

        // State should be consistent after refresh
        assert!(!coordinator.should_quit());
        assert!(coordinator.state.selected_index <= coordinator.sessions.len());
    }

    #[test]
    fn test_integration_service_to_state_to_renderer_pipeline() {
        use crate::test_utils::test_helpers::*;
        use tempfile::TempDir;

        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let config = create_test_config_with_dir(&temp_dir);
        let coordinator = MonitorCoordinator::new(config);

        // Test the pipeline: Service loads sessions → State manages selection → Renderer displays

        // 1. Service component should provide session data
        assert!(coordinator.service.load_sessions(false).is_ok());
        assert!(coordinator.service.load_sessions(true).is_ok());

        // 2. State component should manage selection consistently
        let initial_selection = coordinator.state.selected_index;
        assert!(initial_selection <= coordinator.sessions.len());

        // 3. Renderer should be able to handle the current state
        // We can't test actual rendering without a terminal, but we can verify the renderer exists
        // and the state is consistent for rendering
        assert!(
            coordinator.state.mode != AppMode::ErrorDialog
                || coordinator.state.error_message.is_some()
        );
    }

    #[test]
    fn test_integration_cross_component_error_handling() {
        use crate::test_utils::test_helpers::*;
        use tempfile::TempDir;

        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let config = create_test_config_with_dir(&temp_dir);
        let mut coordinator = MonitorCoordinator::new(config);

        // Test error propagation across components

        // 1. Service errors should not crash the coordinator
        coordinator.refresh_sessions(); // Should handle any service errors gracefully
        assert_eq!(coordinator.state.mode, AppMode::Normal); // Should remain in normal mode

        // 2. State error handling
        coordinator
            .state
            .show_error("Test integration error".to_string());
        assert_eq!(coordinator.state.mode, AppMode::ErrorDialog);
        assert!(coordinator.state.error_message.is_some());

        // 3. Error recovery workflow
        coordinator.state.clear_error();
        assert_eq!(coordinator.state.mode, AppMode::Normal);
        assert!(coordinator.state.error_message.is_none());

        // 4. Coordinator should remain functional after errors
        assert!(!coordinator.should_quit());
        coordinator.refresh_sessions(); // Should not panic
    }

    #[test]
    fn test_integration_session_interaction_workflow() {
        use crate::test_utils::test_helpers::*;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        use tempfile::TempDir;

        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let config = create_test_config_with_dir(&temp_dir);
        let mut coordinator = MonitorCoordinator::new(config);

        // Test complete user interaction workflow

        // 1. Initial navigation
        let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert!(coordinator.handle_key(down_key).is_ok());

        let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert!(coordinator.handle_key(up_key).is_ok());

        // 2. Stale session toggle workflow
        let initial_stale = coordinator.state.show_stale;
        let stale_key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(stale_key).is_ok());
        assert_ne!(coordinator.state.show_stale, initial_stale);

        // Session refresh should have been triggered
        // The sessions list may have changed based on stale filter
        assert!(coordinator.state.selected_index <= coordinator.sessions.len());

        // 3. Modal dialog workflow
        let finish_key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(finish_key).is_ok());
        // Should remain in normal mode if no sessions available

        // 4. Error dialog workflow
        coordinator
            .state
            .show_error("Integration test error".to_string());
        assert_eq!(coordinator.state.mode, AppMode::ErrorDialog);

        let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert!(coordinator.handle_key(enter_key).is_ok());
        assert_eq!(coordinator.state.mode, AppMode::Normal);
    }

    #[test]
    fn test_integration_state_consistency_across_operations() {
        use crate::test_utils::test_helpers::*;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        use tempfile::TempDir;

        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let config = create_test_config_with_dir(&temp_dir);
        let mut coordinator = MonitorCoordinator::new(config);

        // Test state consistency across multiple operations

        // Initial consistency check
        assert!(coordinator.state.selected_index <= coordinator.sessions.len());
        assert_eq!(coordinator.state.mode, AppMode::Normal);

        // Perform multiple operations and verify consistency
        for _ in 0..5 {
            // Navigation
            let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
            coordinator.handle_key(down_key).unwrap();
            assert!(coordinator.state.selected_index <= coordinator.sessions.len());

            // Refresh
            coordinator.refresh_sessions();
            assert!(coordinator.state.selected_index <= coordinator.sessions.len());

            // State toggle
            let stale_key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
            coordinator.handle_key(stale_key).unwrap();
            assert!(coordinator.state.selected_index <= coordinator.sessions.len());
        }

        // Final consistency verification
        assert!(!coordinator.should_quit());
        assert_eq!(coordinator.state.mode, AppMode::Normal);
    }

    #[test]
    fn test_integration_performance_characteristics() {
        use crate::test_utils::test_helpers::*;
        use std::time::Instant;
        use tempfile::TempDir;

        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let config = create_test_config_with_dir(&temp_dir);

        // Test coordinator creation performance
        let start = Instant::now();
        let coordinator = MonitorCoordinator::new(config);
        let creation_time = start.elapsed();

        assert!(
            creation_time.as_millis() < 1000,
            "Coordinator creation should be fast: took {:?}",
            creation_time
        );

        // Test session refresh performance
        let mut coordinator = coordinator;
        let start = Instant::now();
        coordinator.refresh_sessions();
        let refresh_time = start.elapsed();

        assert!(
            refresh_time.as_millis() < 500,
            "Session refresh should be fast: took {:?}",
            refresh_time
        );

        // Test multiple rapid operations
        let start = Instant::now();
        for _ in 0..10 {
            coordinator.refresh_sessions();
        }
        let bulk_refresh_time = start.elapsed();

        assert!(
            bulk_refresh_time.as_millis() < 2000,
            "Bulk operations should be reasonably fast: took {:?}",
            bulk_refresh_time
        );
    }

    #[test]
    fn test_integration_component_isolation_and_boundaries() {
        use crate::test_utils::test_helpers::*;
        use tempfile::TempDir;

        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let config = create_test_config_with_dir(&temp_dir);
        let mut coordinator = MonitorCoordinator::new(config);

        // Test that components maintain proper boundaries and isolation

        // 1. Service component isolation
        // Service should work independently of state changes
        let service_result1 = coordinator.service.load_sessions(false);
        coordinator.state.toggle_stale();
        let service_result2 = coordinator.service.load_sessions(true);

        assert!(service_result1.is_ok());
        assert!(service_result2.is_ok());

        // 2. State component isolation
        // State should maintain consistency regardless of session changes
        let initial_mode = coordinator.state.mode;
        coordinator.refresh_sessions(); // May change session list
        assert_eq!(coordinator.state.mode, initial_mode);

        // 3. Error isolation
        // Errors in one component shouldn't affect others
        coordinator
            .state
            .show_error("Component isolation test".to_string());
        assert_eq!(coordinator.state.mode, AppMode::ErrorDialog);

        // Service should still work
        assert!(coordinator.service.load_sessions(false).is_ok());

        // Clear error and verify recovery
        coordinator.state.clear_error();
        assert_eq!(coordinator.state.mode, AppMode::Normal);

        // 4. Component communication boundaries
        // State changes should propagate properly but not leak implementation details
        let original_sessions_len = coordinator.sessions.len();
        coordinator.refresh_sessions();

        // Sessions may have changed, but coordinator should handle it properly
        assert!(coordinator.state.selected_index <= coordinator.sessions.len());

        // If sessions length changed, selection should be updated appropriately
        if coordinator.sessions.len() != original_sessions_len {
            assert!(coordinator.state.selected_index <= coordinator.sessions.len());
        }
    }

    #[test]
    fn test_integration_concurrent_operations_safety() {
        use crate::test_utils::test_helpers::*;
        use std::sync::{Arc, Mutex};
        use std::thread;
        use tempfile::TempDir;

        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();

        let config = create_test_config_with_dir(&temp_dir);
        let coordinator = Arc::new(Mutex::new(MonitorCoordinator::new(config)));

        // Test that concurrent operations don't cause race conditions or panics
        let handles: Vec<_> = (0..3)
            .map(|i| {
                let coordinator_clone = Arc::clone(&coordinator);
                thread::spawn(move || {
                    for _ in 0..5 {
                        if let Ok(mut coord) = coordinator_clone.try_lock() {
                            // Perform various operations
                            coord.refresh_sessions();
                            coord.state.toggle_stale();

                            // Small delay to simulate real usage
                            thread::sleep(std::time::Duration::from_millis(1));
                        }
                    }
                    i // Return thread ID for verification
                })
            })
            .collect();

        // Wait for all operations to complete
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();

        // Verify all threads completed successfully
        assert_eq!(results.len(), 3);
        assert_eq!(results, vec![0, 1, 2]);

        // Verify coordinator is still in a valid state
        let final_coord = coordinator.lock().unwrap();
        assert!(!final_coord.should_quit());
        assert!(final_coord.state.selected_index <= final_coord.sessions.len());
    }
}
