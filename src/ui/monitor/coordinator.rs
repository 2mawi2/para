use crate::config::Config;
use crate::ui::monitor::action_dispatcher::{ActionDispatcher, ActionResult};
use crate::ui::monitor::actions::MonitorActions;
use crate::ui::monitor::event_handler::EventHandler;
use crate::ui::monitor::renderer::MonitorRenderer;
use crate::ui::monitor::service::SessionService;
use crate::ui::monitor::state::MonitorAppState;
use crate::ui::monitor::state_manager::StateManager;
use crate::ui::monitor::SessionInfo;
use crate::utils::Result;
use crossterm::event::{KeyEvent, MouseEvent, MouseEventKind};
use ratatui::Frame;

/// High-level coordinator for the monitor UI that orchestrates components
pub struct MonitorCoordinator {
    pub state: MonitorAppState,
    pub sessions: Vec<SessionInfo>,
    renderer: MonitorRenderer,
    event_handler: EventHandler,
    action_dispatcher: ActionDispatcher,
    state_manager: StateManager,
}

impl MonitorCoordinator {
    pub fn new(config: Config) -> Self {
        let renderer = MonitorRenderer::new(config.clone());
        let actions = MonitorActions::new(config.clone());
        let service = SessionService::new(config.clone());
        let action_dispatcher = ActionDispatcher::new(actions);
        let event_handler = EventHandler::new();
        let state_manager = StateManager::new(service);
        let state = MonitorAppState::new();

        let mut coordinator = Self {
            state,
            sessions: Vec::new(),
            renderer,
            event_handler,
            action_dispatcher,
            state_manager,
        };

        coordinator.refresh_sessions();
        coordinator
    }

    pub fn refresh_sessions(&mut self) {
        let new_sessions = self.state_manager.load_sessions(self.state.show_stale);
        self.sessions = self
            .state_manager
            .update_sessions(&mut self.state, new_sessions);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if let Some(action) = self
            .event_handler
            .handle_key_event(key, &self.state, &self.sessions)
        {
            self.process_action(action)?;
        }
        Ok(())
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        // Handle selection changes for mouse clicks first
        if let MouseEventKind::Down(crossterm::event::MouseButton::Left) = mouse.kind {
            if let Some(table_area) = self.state.table_area {
                let mouse_x = mouse.column;
                let mouse_y = mouse.row;

                if mouse_x >= table_area.x
                    && mouse_x < table_area.x + table_area.width
                    && mouse_y >= table_area.y
                    && mouse_y < table_area.y + table_area.height
                {
                    let relative_y = mouse_y - table_area.y;
                    if relative_y > 1 {
                        let table_index = (relative_y - 2) as usize;
                        if table_index < self.sessions.len() {
                            // Update selection first
                            self.state_manager.handle_selection_to_index(
                                &mut self.state,
                                table_index,
                                &self.sessions,
                            );
                        }
                    }
                }
            }
        }

        // Then handle any action from the mouse event
        if let Some(action) =
            self.event_handler
                .handle_mouse_event(mouse, &self.state, &self.sessions)
        {
            self.process_action(action)?;
        }
        Ok(())
    }

    /// Process an action by dispatching it and handling the result
    fn process_action(
        &mut self,
        action: crate::ui::monitor::event_handler::UiAction,
    ) -> Result<()> {
        let result = self
            .action_dispatcher
            .dispatch(action, &mut self.state, &self.sessions)?;

        match result {
            ActionResult::RefreshSessions => {
                self.refresh_sessions();
            }
            ActionResult::Continue => {
                // No additional action needed
            }
        }

        Ok(())
    }

    pub fn should_quit(&self) -> bool {
        self.state_manager.should_quit(&self.state)
    }

    pub fn should_refresh(&self) -> bool {
        self.state_manager.should_refresh(&self.state)
    }

    pub fn mark_refreshed(&mut self) {
        self.state_manager.mark_refreshed(&mut self.state);
    }

    pub fn render(&mut self, f: &mut Frame) {
        self.renderer.render(f, &self.sessions, &mut self.state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::monitor::AppMode;
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
        // Sessions may exist in test environment depending on context
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

        // Test finish mode - only changes mode if sessions exist
        if !coordinator.sessions.is_empty() {
            coordinator.state.start_finish();
            assert_eq!(coordinator.state.mode, AppMode::FinishPrompt);

            // Reset to normal mode for cancel test
            coordinator.state.exit_dialog();
            assert_eq!(coordinator.state.mode, AppMode::Normal);

            // Test cancel mode
            coordinator.state.start_cancel();
            assert_eq!(coordinator.state.mode, AppMode::CancelConfirm);
        }
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

        // Mark as refreshed first to ensure we have a known starting point
        coordinator.mark_refreshed();

        // Should not need refresh immediately after marking
        assert!(!coordinator.should_refresh());

        // Sleep for a short time (less than 2 seconds)
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Should still not need refresh
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

    #[test]
    fn test_handle_mouse_click() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::layout::Rect;

        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Simulate a table area (x=1, y=4, width=100, height=20)
        // This simulates where the table would be rendered
        coordinator.state.set_table_area(Rect {
            x: 1,
            y: 4,
            width: 100,
            height: 20,
        });

        // Simulate having some sessions
        if !coordinator.sessions.is_empty() {
            // Test clicking on the first data row (y=6 accounts for header and border)
            let mouse_event = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 10,
                row: 6, // First data row (4 + 2 for header and border)
                modifiers: crossterm::event::KeyModifiers::NONE,
            };

            assert!(coordinator.handle_mouse(mouse_event).is_ok());
            assert_eq!(coordinator.state.selected_index, 0);

            // Test clicking on a row beyond sessions (should not change selection)
            let out_of_bounds_event = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 10,
                row: 100,
                modifiers: crossterm::event::KeyModifiers::NONE,
            };

            let prev_index = coordinator.state.selected_index;
            assert!(coordinator.handle_mouse(out_of_bounds_event).is_ok());
            assert_eq!(coordinator.state.selected_index, prev_index);

            // Test clicking outside the table area horizontally (should not change selection)
            let outside_table_event = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 200, // Outside the table width
                row: 6,
                modifiers: crossterm::event::KeyModifiers::NONE,
            };

            let prev_index = coordinator.state.selected_index;
            assert!(coordinator.handle_mouse(outside_table_event).is_ok());
            assert_eq!(coordinator.state.selected_index, prev_index);

            // Test clicking on the header row (should not change selection)
            let header_click_event = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 10,
                row: 4, // Header row
                modifiers: crossterm::event::KeyModifiers::NONE,
            };

            let prev_index = coordinator.state.selected_index;
            assert!(coordinator.handle_mouse(header_click_event).is_ok());
            assert_eq!(coordinator.state.selected_index, prev_index);
        }

        // Test mouse events in dialog mode (should be ignored)
        coordinator.state.mode = AppMode::FinishPrompt;
        let dialog_mouse_event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 6,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        assert!(coordinator.handle_mouse(dialog_mouse_event).is_ok());
        // Mode should remain unchanged
        assert_eq!(coordinator.state.mode, AppMode::FinishPrompt);
    }

    // === COMPREHENSIVE MOUSE EVENT TESTS ===

    #[test]
    fn test_mouse_click_resume_button() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::layout::Rect;

        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Create mock session data
        let mock_session = SessionInfo {
            name: "test-session".to_string(),
            branch: "test-branch".to_string(),
            status: crate::ui::monitor::SessionStatus::Active,
            last_activity: chrono::Utc::now(),
            task: "Test task".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/test"),
            test_status: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        };
        coordinator.sessions = vec![mock_session];

        // Set up table area
        coordinator.state.set_table_area(Rect {
            x: 1,
            y: 4,
            width: 100,
            height: 20,
        });

        // Click on the first session row in the resume button area (positions 0-2)
        let resume_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2, // Within resume button area (0-2)
            row: 6,    // First data row (4 + 2 for header and border)
            modifiers: crossterm::event::KeyModifiers::NONE,
        };

        assert!(coordinator.handle_mouse(resume_click).is_ok());
        assert_eq!(coordinator.state.selected_index, 0);

        // Verify button click was registered
        assert!(coordinator.state.get_button_click().is_some());
        if let Some(click) = coordinator.state.get_button_click() {
            assert_eq!(*click, crate::ui::monitor::state::ButtonClick::Resume(0));
        }
    }

    #[test]
    fn test_mouse_click_copy_button() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::layout::Rect;

        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Create mock session data
        let mock_session = SessionInfo {
            name: "test-session".to_string(),
            branch: "test-branch".to_string(),
            status: crate::ui::monitor::SessionStatus::Active,
            last_activity: chrono::Utc::now(),
            task: "Test task".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/test"),
            test_status: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        };
        coordinator.sessions = vec![mock_session];

        // Set up table area
        coordinator.state.set_table_area(Rect {
            x: 1,
            y: 4,
            width: 100,
            height: 20,
        });

        // Click on the first session row in the copy button area (positions 12-15)
        let copy_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 13, // Within copy button area (12-15)
            row: 6,     // First data row (4 + 2 for header and border)
            modifiers: crossterm::event::KeyModifiers::NONE,
        };

        assert!(coordinator.handle_mouse(copy_click).is_ok());
        assert_eq!(coordinator.state.selected_index, 0);

        // Verify button click was registered
        assert!(coordinator.state.get_button_click().is_some());
        if let Some(click) = coordinator.state.get_button_click() {
            assert_eq!(*click, crate::ui::monitor::state::ButtonClick::Copy(0));
        }
    }

    #[test]
    fn test_mouse_click_outside_table() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::layout::Rect;

        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Create mock session data
        let mock_session = SessionInfo {
            name: "test-session".to_string(),
            branch: "test-branch".to_string(),
            status: crate::ui::monitor::SessionStatus::Active,
            last_activity: chrono::Utc::now(),
            task: "Test task".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/test"),
            test_status: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        };
        coordinator.sessions = vec![mock_session];

        // Set up table area
        coordinator.state.set_table_area(Rect {
            x: 1,
            y: 4,
            width: 100,
            height: 20,
        });

        let initial_index = coordinator.state.selected_index;

        // Click outside table bounds (horizontally)
        let outside_horizontal = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 150, // Outside table width (1 + 100)
            row: 6,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };

        assert!(coordinator.handle_mouse(outside_horizontal).is_ok());
        assert_eq!(coordinator.state.selected_index, initial_index);

        // Click outside table bounds (vertically)
        let outside_vertical = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 30, // Outside table height (4 + 20)
            modifiers: crossterm::event::KeyModifiers::NONE,
        };

        assert!(coordinator.handle_mouse(outside_vertical).is_ok());
        assert_eq!(coordinator.state.selected_index, initial_index);

        // Click before table area
        let before_table = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0, // Before table x position (1)
            row: 6,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };

        assert!(coordinator.handle_mouse(before_table).is_ok());
        assert_eq!(coordinator.state.selected_index, initial_index);
    }

    #[test]
    fn test_mouse_click_on_table_header() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::layout::Rect;

        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Create mock session data
        let mock_session = SessionInfo {
            name: "test-session".to_string(),
            branch: "test-branch".to_string(),
            status: crate::ui::monitor::SessionStatus::Active,
            last_activity: chrono::Utc::now(),
            task: "Test task".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/test"),
            test_status: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        };
        coordinator.sessions = vec![mock_session];

        // Set up table area
        coordinator.state.set_table_area(Rect {
            x: 1,
            y: 4,
            width: 100,
            height: 20,
        });

        let initial_index = coordinator.state.selected_index;

        // Click on header row (row 4)
        let header_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 4, // Header row
            modifiers: crossterm::event::KeyModifiers::NONE,
        };

        assert!(coordinator.handle_mouse(header_click).is_ok());
        assert_eq!(coordinator.state.selected_index, initial_index);

        // Click on border row (row 5)
        let border_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 5, // Border row
            modifiers: crossterm::event::KeyModifiers::NONE,
        };

        assert!(coordinator.handle_mouse(border_click).is_ok());
        assert_eq!(coordinator.state.selected_index, initial_index);
    }

    // === COMPREHENSIVE KEYBOARD NAVIGATION TESTS ===

    #[test]
    fn test_normal_mode_navigation() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Create multiple mock sessions
        let sessions = vec![
            SessionInfo {
                name: "session1".to_string(),
                branch: "branch1".to_string(),
                status: crate::ui::monitor::SessionStatus::Active,
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
                status: crate::ui::monitor::SessionStatus::Idle,
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
                status: crate::ui::monitor::SessionStatus::Ready,
                last_activity: chrono::Utc::now(),
                task: "Task 3".to_string(),
                worktree_path: std::path::PathBuf::from("/tmp/session3"),
                test_status: None,
                diff_stats: None,
                todo_percentage: None,
                is_blocked: false,
            },
        ];
        coordinator.sessions = sessions;

        // Test basic navigation
        assert_eq!(coordinator.state.selected_index, 0);

        // Test down navigation (j key)
        let down_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(down_j).is_ok());
        assert_eq!(coordinator.state.selected_index, 1);

        // Test down navigation (Down arrow)
        let down_arrow = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert!(coordinator.handle_key(down_arrow).is_ok());
        assert_eq!(coordinator.state.selected_index, 2);

        // Test boundary (shouldn't go beyond last item)
        let down_boundary = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(down_boundary).is_ok());
        assert_eq!(coordinator.state.selected_index, 2);

        // Test up navigation (k key)
        let up_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(up_k).is_ok());
        assert_eq!(coordinator.state.selected_index, 1);

        // Test up navigation (Up arrow)
        let up_arrow = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert!(coordinator.handle_key(up_arrow).is_ok());
        assert_eq!(coordinator.state.selected_index, 0);

        // Test boundary (shouldn't go below 0)
        let up_boundary = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(up_boundary).is_ok());
        assert_eq!(coordinator.state.selected_index, 0);

        // Test stale toggle
        let initial_stale = coordinator.state.show_stale;
        let stale_toggle = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(stale_toggle).is_ok());
        assert_eq!(coordinator.state.show_stale, !initial_stale);

        // Test quit keys
        let mut test_coordinator = MonitorCoordinator::new(create_test_config());
        let quit_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(test_coordinator.handle_key(quit_q).is_ok());
        assert!(test_coordinator.should_quit());

        let mut test_coordinator2 = MonitorCoordinator::new(create_test_config());
        let quit_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert!(test_coordinator2.handle_key(quit_esc).is_ok());
        assert!(test_coordinator2.should_quit());

        // Test Ctrl+C
        let mut test_coordinator3 = MonitorCoordinator::new(create_test_config());
        let quit_ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(test_coordinator3.handle_key(quit_ctrl_c).is_ok());
        assert!(test_coordinator3.should_quit());
    }

    #[test]
    fn test_finish_prompt_mode_input() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Create mock session
        let mock_session = SessionInfo {
            name: "test-session".to_string(),
            branch: "test-branch".to_string(),
            status: crate::ui::monitor::SessionStatus::Active,
            last_activity: chrono::Utc::now(),
            task: "Test task".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/test"),
            test_status: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        };
        coordinator.sessions = vec![mock_session];

        // Start finish mode
        coordinator.state.start_finish();
        assert_eq!(coordinator.state.mode, AppMode::FinishPrompt);

        // Test character input
        let char_t = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(char_t).is_ok());
        assert_eq!(coordinator.state.get_input(), "t");

        let char_e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(char_e).is_ok());
        assert_eq!(coordinator.state.get_input(), "te");

        let char_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(char_s).is_ok());
        assert_eq!(coordinator.state.get_input(), "tes");

        let char_t2 = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(char_t2).is_ok());
        assert_eq!(coordinator.state.get_input(), "test");

        // Test backspace
        let backspace = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        assert!(coordinator.handle_key(backspace).is_ok());
        assert_eq!(coordinator.state.get_input(), "tes");

        // Test escape to exit
        let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert!(coordinator.handle_key(escape).is_ok());
        assert_eq!(coordinator.state.mode, AppMode::Normal);
        assert_eq!(coordinator.state.get_input(), "");

        // Test Ctrl+C to exit
        coordinator.state.start_finish();
        coordinator.state.add_char('t');
        coordinator.state.add_char('e');
        coordinator.state.add_char('s');
        coordinator.state.add_char('t');
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(coordinator.handle_key(ctrl_c).is_ok());
        assert_eq!(coordinator.state.mode, AppMode::Normal);
        assert_eq!(coordinator.state.get_input(), "");
    }

    #[test]
    fn test_mode_switching() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Create mock session
        let mock_session = SessionInfo {
            name: "test-session".to_string(),
            branch: "test-branch".to_string(),
            status: crate::ui::monitor::SessionStatus::Active,
            last_activity: chrono::Utc::now(),
            task: "Test task".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/test"),
            test_status: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        };
        coordinator.sessions = vec![mock_session];

        // Test starting in Normal mode
        assert_eq!(coordinator.state.mode, AppMode::Normal);

        // Test switching to FinishPrompt mode
        let finish_key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(finish_key).is_ok());
        assert_eq!(coordinator.state.mode, AppMode::FinishPrompt);

        // Test exiting back to Normal mode
        let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert!(coordinator.handle_key(escape).is_ok());
        assert_eq!(coordinator.state.mode, AppMode::Normal);

        // Test switching to CancelConfirm mode
        let cancel_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        assert!(coordinator.handle_key(cancel_key).is_ok());
        assert_eq!(coordinator.state.mode, AppMode::CancelConfirm);

        // Test exiting back to Normal mode
        let escape2 = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert!(coordinator.handle_key(escape2).is_ok());
        assert_eq!(coordinator.state.mode, AppMode::Normal);

        // Test ErrorDialog mode
        coordinator.state.show_error("Test error".to_string());
        assert_eq!(coordinator.state.mode, AppMode::ErrorDialog);

        // Test dismissing error with Enter
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert!(coordinator.handle_key(enter).is_ok());
        assert_eq!(coordinator.state.mode, AppMode::Normal);

        // Test ErrorDialog mode again
        coordinator.state.show_error("Test error 2".to_string());
        assert_eq!(coordinator.state.mode, AppMode::ErrorDialog);

        // Test dismissing error with Space
        let space = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        assert!(coordinator.handle_key(space).is_ok());
        assert_eq!(coordinator.state.mode, AppMode::Normal);
    }

    // === COMPREHENSIVE STATE MANAGEMENT TESTS ===

    #[test]
    fn test_session_selection_updates() {
        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Test with no sessions
        coordinator.sessions = vec![];
        coordinator
            .state
            .update_selection_for_sessions(&coordinator.sessions);
        assert_eq!(coordinator.state.selected_index, 0);
        assert_eq!(coordinator.state.table_state.selected(), None);

        // Test with one session
        let session1 = SessionInfo {
            name: "session1".to_string(),
            branch: "branch1".to_string(),
            status: crate::ui::monitor::SessionStatus::Active,
            last_activity: chrono::Utc::now(),
            task: "Task 1".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/session1"),
            test_status: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        };
        coordinator.sessions = vec![session1];
        coordinator
            .state
            .update_selection_for_sessions(&coordinator.sessions);
        assert_eq!(coordinator.state.selected_index, 0);
        assert_eq!(coordinator.state.table_state.selected(), Some(0));

        // Test with selection beyond bounds
        coordinator.state.selected_index = 5;
        coordinator
            .state
            .update_selection_for_sessions(&coordinator.sessions);
        assert_eq!(coordinator.state.selected_index, 0); // Should clamp to last valid index

        // Test with multiple sessions
        let session2 = SessionInfo {
            name: "session2".to_string(),
            branch: "branch2".to_string(),
            status: crate::ui::monitor::SessionStatus::Idle,
            last_activity: chrono::Utc::now(),
            task: "Task 2".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/session2"),
            test_status: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        };
        coordinator.sessions.push(session2);

        // Test selection within valid range
        coordinator.state.selected_index = 1;
        coordinator
            .state
            .update_selection_for_sessions(&coordinator.sessions);
        assert_eq!(coordinator.state.selected_index, 1);
        assert_eq!(coordinator.state.table_state.selected(), Some(1));

        // Test selection beyond new bounds
        coordinator.state.selected_index = 5;
        coordinator
            .state
            .update_selection_for_sessions(&coordinator.sessions);
        assert_eq!(coordinator.state.selected_index, 1); // Should clamp to last valid index
    }

    #[test]
    fn test_refresh_sessions() {
        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Test refresh doesn't crash and maintains state
        coordinator.refresh_sessions();

        // Should update selection for new session list
        assert!(coordinator.state.selected_index <= coordinator.sessions.len());

        // Test refresh with different stale settings
        let initial_stale = coordinator.state.show_stale;
        coordinator.state.toggle_stale();
        coordinator.refresh_sessions();
        assert_eq!(coordinator.state.show_stale, !initial_stale);

        // Test refresh maintains table state consistency
        if !coordinator.sessions.is_empty() {
            assert!(coordinator.state.table_state.selected().is_some());
        } else {
            assert!(coordinator.state.table_state.selected().is_none());
        }
    }

    #[test]
    fn test_table_area_updates() {
        use ratatui::layout::Rect;

        let config = create_test_config();
        let mut coordinator = MonitorCoordinator::new(config);

        // Test initial state (no table area)
        assert!(coordinator.state.table_area.is_none());

        // Test setting table area
        let test_area = Rect {
            x: 5,
            y: 10,
            width: 80,
            height: 15,
        };
        coordinator.state.set_table_area(test_area);
        assert!(coordinator.state.table_area.is_some());
        assert_eq!(coordinator.state.table_area.unwrap(), test_area);

        // Test updating table area
        let new_area = Rect {
            x: 0,
            y: 0,
            width: 120,
            height: 30,
        };
        coordinator.state.set_table_area(new_area);
        assert_eq!(coordinator.state.table_area.unwrap(), new_area);

        // Test table area affects mouse handling
        let mock_session = SessionInfo {
            name: "test-session".to_string(),
            branch: "test-branch".to_string(),
            status: crate::ui::monitor::SessionStatus::Active,
            last_activity: chrono::Utc::now(),
            task: "Test task".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/test"),
            test_status: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        };
        coordinator.sessions = vec![mock_session];

        // Mouse click within new area should work
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        let click_in_area = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 2, // First data row in new area (0 + 2 for header)
            modifiers: crossterm::event::KeyModifiers::NONE,
        };

        assert!(coordinator.handle_mouse(click_in_area).is_ok());
        assert_eq!(coordinator.state.selected_index, 0);

        // Mouse click outside new area should not work
        let click_outside = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 150, // Outside width
            row: 2,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };

        let prev_selection = coordinator.state.selected_index;
        assert!(coordinator.handle_mouse(click_outside).is_ok());
        assert_eq!(coordinator.state.selected_index, prev_selection);
    }
}
