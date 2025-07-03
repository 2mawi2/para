use crate::ui::monitor::state::MonitorAppState;
use crate::ui::monitor::{AppMode, SessionInfo};
use crossterm::event::{KeyEvent, MouseEvent, MouseEventKind};

/// Actions that can be triggered by UI events
#[derive(Debug, Clone, PartialEq)]
pub enum UiAction {
    Session(SessionAction),
    Navigation(NavigationAction),
    Dialog(DialogAction),
    System(SystemAction),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionAction {
    Resume(usize),
    Copy(usize),
    Integrate(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum NavigationAction {
    SelectNext,
    SelectPrevious,
    ToggleStale,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DialogAction {
    StartFinish,
    StartCancel,
    ExitDialog,
    AddChar(char),
    Backspace,
    ExecuteFinish,
    ExecuteCancel,
    ClearError,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SystemAction {
    Quit,
}

/// Handles input events and translates them to UI actions
#[derive(Default)]
pub struct EventHandler;

impl EventHandler {
    pub fn new() -> Self {
        Self
    }

    /// Handle a keyboard event and return the corresponding action
    pub fn handle_key_event(
        &self,
        key: KeyEvent,
        state: &MonitorAppState,
        sessions: &[SessionInfo],
    ) -> Option<UiAction> {
        match state.mode {
            AppMode::Normal => self.handle_normal_key(key, state, sessions),
            AppMode::FinishPrompt => self.handle_finish_prompt_key(key, state),
            AppMode::CancelConfirm => self.handle_cancel_confirm_key(key),
            AppMode::ErrorDialog => self.handle_error_dialog_key(key),
        }
    }

    /// Handle a mouse event and return the corresponding action
    pub fn handle_mouse_event(
        &self,
        mouse: MouseEvent,
        state: &MonitorAppState,
        sessions: &[SessionInfo],
    ) -> Option<UiAction> {
        match state.mode {
            AppMode::Normal => self.handle_normal_mouse(mouse, state, sessions),
            AppMode::FinishPrompt | AppMode::CancelConfirm | AppMode::ErrorDialog => {
                // Ignore mouse events in dialog modes
                None
            }
        }
    }

    fn handle_normal_key(
        &self,
        key: KeyEvent,
        state: &MonitorAppState,
        sessions: &[SessionInfo],
    ) -> Option<UiAction> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(UiAction::System(SystemAction::Quit)),
            KeyCode::Char('c') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    Some(UiAction::System(SystemAction::Quit))
                } else if state.get_selected_session(sessions).is_some() {
                    Some(UiAction::Dialog(DialogAction::StartCancel))
                } else {
                    None
                }
            }
            KeyCode::Char('y') => {
                // 'y' to yank/copy session name (like vim)
                if state.get_selected_session(sessions).is_some() {
                    Some(UiAction::Session(SessionAction::Copy(state.selected_index)))
                } else {
                    None
                }
            }
            KeyCode::Char('s') => Some(UiAction::Navigation(NavigationAction::ToggleStale)),
            KeyCode::Up | KeyCode::Char('k') => {
                Some(UiAction::Navigation(NavigationAction::SelectPrevious))
            }
            KeyCode::Down | KeyCode::Char('j') => {
                Some(UiAction::Navigation(NavigationAction::SelectNext))
            }
            KeyCode::Enter => {
                if state.get_selected_session(sessions).is_some() {
                    Some(UiAction::Session(SessionAction::Resume(
                        state.selected_index,
                    )))
                } else {
                    None
                }
            }
            KeyCode::Tab => {
                // Tab navigation between buttons could be implemented here
                // For now, no action
                None
            }
            KeyCode::Char('f') => {
                if state.get_selected_session(sessions).is_some() {
                    Some(UiAction::Dialog(DialogAction::StartFinish))
                } else {
                    None
                }
            }
            KeyCode::Char('i') => {
                if state.get_selected_session(sessions).is_some() {
                    Some(UiAction::Session(SessionAction::Integrate(
                        state.selected_index,
                    )))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn handle_finish_prompt_key(&self, key: KeyEvent, state: &MonitorAppState) -> Option<UiAction> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Esc => Some(UiAction::Dialog(DialogAction::ExitDialog)),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(UiAction::Dialog(DialogAction::ExitDialog))
            }
            KeyCode::Enter => {
                if state.is_input_ready() {
                    Some(UiAction::Dialog(DialogAction::ExecuteFinish))
                } else {
                    None
                }
            }
            KeyCode::Backspace => Some(UiAction::Dialog(DialogAction::Backspace)),
            KeyCode::Char(c) => Some(UiAction::Dialog(DialogAction::AddChar(c))),
            _ => None,
        }
    }

    fn handle_cancel_confirm_key(&self, key: KeyEvent) -> Option<UiAction> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Enter => Some(UiAction::Dialog(DialogAction::ExecuteCancel)),
            KeyCode::Esc => Some(UiAction::Dialog(DialogAction::ExitDialog)),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(UiAction::Dialog(DialogAction::ExitDialog))
            }
            _ => None,
        }
    }

    fn handle_error_dialog_key(&self, key: KeyEvent) -> Option<UiAction> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
                Some(UiAction::Dialog(DialogAction::ClearError))
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(UiAction::Dialog(DialogAction::ClearError))
            }
            _ => None,
        }
    }

    fn handle_normal_mouse(
        &self,
        mouse: MouseEvent,
        state: &MonitorAppState,
        sessions: &[SessionInfo],
    ) -> Option<UiAction> {
        // Only handle left clicks
        if !matches!(
            mouse.kind,
            MouseEventKind::Down(crossterm::event::MouseButton::Left)
        ) {
            return None;
        }

        // Check if we have a stored table area
        let table_area = state.table_area?;

        // Check if the click is within the table area
        if !self.is_click_in_table_area(mouse, table_area) {
            return None;
        }

        // Calculate table position (row index and relative x)
        let (table_index, relative_x) = self.calculate_table_position(mouse, table_area)?;

        // Check if the clicked row is within the session list bounds
        if table_index >= sessions.len() {
            return None;
        }

        // Check if clicking in the actions column (first 9 characters)
        if relative_x < 9 {
            // Try to detect button action
            if let Some(action) = self.detect_button_action(relative_x, table_index) {
                return Some(UiAction::Session(action));
            }
        }

        // If clicking elsewhere on the row, just select it (no additional action beyond selection)
        // Selection will be handled by the caller
        None
    }

    /// Check if a mouse click is within the table area
    fn is_click_in_table_area(&self, mouse: MouseEvent, table_area: ratatui::layout::Rect) -> bool {
        let mouse_x = mouse.column;
        let mouse_y = mouse.row;

        mouse_x >= table_area.x
            && mouse_x < table_area.x + table_area.width
            && mouse_y >= table_area.y
            && mouse_y < table_area.y + table_area.height
    }

    /// Calculate the table position (row index and relative x) from a mouse event
    /// Returns None if the click is on header or border rows
    fn calculate_table_position(
        &self,
        mouse: MouseEvent,
        table_area: ratatui::layout::Rect,
    ) -> Option<(usize, u16)> {
        let relative_y = mouse.row - table_area.y;
        let relative_x = mouse.column - table_area.x;

        // Skip if clicking on the header row (row 0) or the border (row 1)
        if relative_y > 1 {
            // Subtract 2 for header and border to get the data row index
            let table_index = (relative_y - 2) as usize;
            Some((table_index, relative_x))
        } else {
            None
        }
    }

    /// Detect which button action should be performed based on relative x position
    /// Returns None if the click is not on a button
    fn detect_button_action(&self, relative_x: u16, session_index: usize) -> Option<SessionAction> {
        // Actions column: "[▶] [📋]" (positions 0-8)
        // [▶] = positions 0-2
        // space = position 3
        // [📋] = positions 4-7
        if relative_x < 3 {
            // Resume button clicked
            Some(SessionAction::Resume(session_index))
        } else if (4..8).contains(&relative_x) {
            // Copy button clicked
            Some(SessionAction::Copy(session_index))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
    use ratatui::layout::Rect;

    fn create_test_state() -> MonitorAppState {
        MonitorAppState::new()
    }

    fn create_test_sessions() -> Vec<SessionInfo> {
        vec![
            SessionInfo {
                name: "session1".to_string(),
                branch: "branch1".to_string(),
                status: crate::ui::monitor::SessionStatus::Active,
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
                status: crate::ui::monitor::SessionStatus::Idle,
                last_activity: chrono::Utc::now(),
                task: "Task 2".to_string(),
                worktree_path: std::path::PathBuf::from("/tmp/session2"),
                test_status: None,
                confidence: None,
                diff_stats: None,
                todo_percentage: None,
                is_blocked: false,
            },
        ]
    }

    #[test]
    fn test_normal_mode_key_handling() {
        let event_handler = EventHandler::new();
        let state = create_test_state();
        let sessions = create_test_sessions();

        // Test quit keys
        let quit_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(quit_q, &state, &sessions),
            Some(UiAction::System(SystemAction::Quit))
        );

        let quit_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(quit_esc, &state, &sessions),
            Some(UiAction::System(SystemAction::Quit))
        );

        let quit_ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(
            event_handler.handle_key_event(quit_ctrl_c, &state, &sessions),
            Some(UiAction::System(SystemAction::Quit))
        );

        // Test navigation
        let nav_up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(nav_up, &state, &sessions),
            Some(UiAction::Navigation(NavigationAction::SelectPrevious))
        );

        let nav_down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(nav_down, &state, &sessions),
            Some(UiAction::Navigation(NavigationAction::SelectNext))
        );

        // Test stale toggle
        let stale_toggle = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(stale_toggle, &state, &sessions),
            Some(UiAction::Navigation(NavigationAction::ToggleStale))
        );

        // Test session actions
        let resume_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(resume_key, &state, &sessions),
            Some(UiAction::Session(SessionAction::Resume(0)))
        );

        let copy_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(copy_key, &state, &sessions),
            Some(UiAction::Session(SessionAction::Copy(0)))
        );

        let finish_key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(finish_key, &state, &sessions),
            Some(UiAction::Dialog(DialogAction::StartFinish))
        );

        let cancel_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(cancel_key, &state, &sessions),
            Some(UiAction::Dialog(DialogAction::StartCancel))
        );
    }

    #[test]
    fn test_finish_prompt_mode_key_handling() {
        let event_handler = EventHandler::new();
        let mut state = create_test_state();
        state.start_finish();
        let sessions = create_test_sessions();

        // Test character input
        let char_key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(char_key, &state, &sessions),
            Some(UiAction::Dialog(DialogAction::AddChar('t')))
        );

        // Test backspace
        let backspace_key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(backspace_key, &state, &sessions),
            Some(UiAction::Dialog(DialogAction::Backspace))
        );

        // Test exit dialog
        let escape_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(escape_key, &state, &sessions),
            Some(UiAction::Dialog(DialogAction::ExitDialog))
        );

        // Test execute finish (when input is ready)
        state.add_char('t');
        state.add_char('e');
        state.add_char('s');
        state.add_char('t');
        let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(enter_key, &state, &sessions),
            Some(UiAction::Dialog(DialogAction::ExecuteFinish))
        );
    }

    #[test]
    fn test_error_dialog_key_handling() {
        let event_handler = EventHandler::new();
        let mut state = create_test_state();
        state.show_error("Test error".to_string());
        let sessions = create_test_sessions();

        // Test clear error with different keys
        let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(enter_key, &state, &sessions),
            Some(UiAction::Dialog(DialogAction::ClearError))
        );

        let escape_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(escape_key, &state, &sessions),
            Some(UiAction::Dialog(DialogAction::ClearError))
        );

        let space_key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(space_key, &state, &sessions),
            Some(UiAction::Dialog(DialogAction::ClearError))
        );
    }

    #[test]
    fn test_mouse_event_handling() {
        let event_handler = EventHandler::new();
        let mut state = create_test_state();
        let sessions = create_test_sessions();

        // Set up table area
        state.set_table_area(Rect {
            x: 1,
            y: 4,
            width: 100,
            height: 20,
        });

        // Test clicking resume button
        let resume_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 2, // Within resume button area (0-2)
            row: 6,    // First data row (4 + 2 for header and border)
            modifiers: KeyModifiers::NONE,
        };

        assert_eq!(
            event_handler.handle_mouse_event(resume_click, &state, &sessions),
            Some(UiAction::Session(SessionAction::Resume(0)))
        );

        // Test clicking copy button
        let copy_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5, // Within copy button area (4-7)
            row: 6,    // First data row
            modifiers: KeyModifiers::NONE,
        };

        assert_eq!(
            event_handler.handle_mouse_event(copy_click, &state, &sessions),
            Some(UiAction::Session(SessionAction::Copy(0)))
        );

        // Test clicking outside table area - should return None
        let outside_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 150, // Outside table area
            row: 6,
            modifiers: KeyModifiers::NONE,
        };

        assert_eq!(
            event_handler.handle_mouse_event(outside_click, &state, &sessions),
            None
        );
    }

    #[test]
    fn test_empty_sessions_behavior() {
        let event_handler = EventHandler::new();
        let state = create_test_state();
        let empty_sessions = vec![];

        // Actions that require sessions should return None
        let resume_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(resume_key, &state, &empty_sessions),
            None
        );

        let copy_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(copy_key, &state, &empty_sessions),
            None
        );

        let finish_key = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(finish_key, &state, &empty_sessions),
            None
        );

        let cancel_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(cancel_key, &state, &empty_sessions),
            None
        );

        // Navigation actions should still work
        let stale_toggle = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(
            event_handler.handle_key_event(stale_toggle, &state, &empty_sessions),
            Some(UiAction::Navigation(NavigationAction::ToggleStale))
        );
    }

    #[test]
    fn test_is_click_in_table_area() {
        let event_handler = EventHandler::new();
        let table_area = Rect {
            x: 1,
            y: 4,
            width: 100,
            height: 20,
        };

        // Test click inside table area
        let inside_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 50, // Within table area (1-101)
            row: 10,    // Within table area (4-24)
            modifiers: KeyModifiers::NONE,
        };
        assert!(event_handler.is_click_in_table_area(inside_click, table_area));

        // Test click outside table area - left edge
        let outside_left = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0, // Outside table area
            row: 10,
            modifiers: KeyModifiers::NONE,
        };
        assert!(!event_handler.is_click_in_table_area(outside_left, table_area));

        // Test click outside table area - right edge
        let outside_right = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 101, // Outside table area
            row: 10,
            modifiers: KeyModifiers::NONE,
        };
        assert!(!event_handler.is_click_in_table_area(outside_right, table_area));

        // Test click outside table area - above
        let outside_above = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 50,
            row: 3, // Outside table area
            modifiers: KeyModifiers::NONE,
        };
        assert!(!event_handler.is_click_in_table_area(outside_above, table_area));

        // Test click outside table area - below
        let outside_below = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 50,
            row: 24, // Outside table area
            modifiers: KeyModifiers::NONE,
        };
        assert!(!event_handler.is_click_in_table_area(outside_below, table_area));
    }

    #[test]
    fn test_calculate_table_position() {
        let event_handler = EventHandler::new();
        let table_area = Rect {
            x: 1,
            y: 4,
            width: 100,
            height: 20,
        };

        // Test valid position calculation
        let valid_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 6, // Relative position 5 from table start
            row: 7,    // Relative position 3 from table start, minus 2 for header = row 1
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(
            event_handler.calculate_table_position(valid_click, table_area),
            Some((1, 5)) // (row_index, relative_x)
        );

        // Test click on header row (should return None)
        let header_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 6,
            row: 4, // Header row
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(
            event_handler.calculate_table_position(header_click, table_area),
            None
        );

        // Test click on border row (should return None)
        let border_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 6,
            row: 5, // Border row
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(
            event_handler.calculate_table_position(border_click, table_area),
            None
        );
    }

    #[test]
    fn test_detect_button_action() {
        let event_handler = EventHandler::new();

        // Test resume button (positions 0-2)
        assert_eq!(
            event_handler.detect_button_action(0, 0),
            Some(SessionAction::Resume(0))
        );
        assert_eq!(
            event_handler.detect_button_action(1, 0),
            Some(SessionAction::Resume(0))
        );
        assert_eq!(
            event_handler.detect_button_action(2, 0),
            Some(SessionAction::Resume(0))
        );

        // Test copy button (positions 4-7)
        assert_eq!(
            event_handler.detect_button_action(4, 0),
            Some(SessionAction::Copy(0))
        );
        assert_eq!(
            event_handler.detect_button_action(5, 0),
            Some(SessionAction::Copy(0))
        );
        assert_eq!(
            event_handler.detect_button_action(6, 0),
            Some(SessionAction::Copy(0))
        );
        assert_eq!(
            event_handler.detect_button_action(7, 0),
            Some(SessionAction::Copy(0))
        );

        // Test positions outside button areas
        assert_eq!(event_handler.detect_button_action(3, 0), None); // Space between buttons
        assert_eq!(event_handler.detect_button_action(8, 0), None); // Beyond buttons
        assert_eq!(event_handler.detect_button_action(9, 0), None); // Beyond buttons

        // Test with different session index
        assert_eq!(
            event_handler.detect_button_action(0, 1),
            Some(SessionAction::Resume(1))
        );
        assert_eq!(
            event_handler.detect_button_action(5, 2),
            Some(SessionAction::Copy(2))
        );
    }
}
