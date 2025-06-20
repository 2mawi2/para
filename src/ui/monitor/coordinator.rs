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
    use crate::ui::monitor::test_utils::create_test_config;
    use crossterm::event::{KeyCode, KeyEvent};


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
}
