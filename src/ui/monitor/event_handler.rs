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

/// Represents mouse click coordinates
#[derive(Debug, Clone)]
struct ClickInfo {
    x: u16,
    y: u16,
}

/// Represents relative coordinates within a table
#[derive(Debug, Clone)]
struct TableCoords {
    relative_x: u16,
    relative_y: u16,
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
        let click_info = self.extract_left_click(&mouse)?;
        let table_area = state.table_area?;
        let coords = self.validate_click_in_table(&click_info, &table_area)?;
        let row_index = self.calculate_table_row_index(&coords)?;

        if row_index >= sessions.len() {
            return None;
        }

        self.detect_button_click(&coords, row_index)
    }

    fn extract_left_click(&self, mouse: &MouseEvent) -> Option<ClickInfo> {
        if let MouseEventKind::Down(crossterm::event::MouseButton::Left) = mouse.kind {
            Some(ClickInfo {
                x: mouse.column,
                y: mouse.row,
            })
        } else {
            None
        }
    }

    fn validate_click_in_table(
        &self,
        click: &ClickInfo,
        table_area: &ratatui::layout::Rect,
    ) -> Option<TableCoords> {
        if click.x >= table_area.x
            && click.x < table_area.x + table_area.width
            && click.y >= table_area.y
            && click.y < table_area.y + table_area.height
        {
            Some(TableCoords {
                relative_x: click.x - table_area.x,
                relative_y: click.y - table_area.y,
            })
        } else {
            None
        }
    }

    fn calculate_table_row_index(&self, coords: &TableCoords) -> Option<usize> {
        // Skip if clicking on the header row (row 0) or the border (row 1)
        if coords.relative_y > 1 {
            // Subtract 2 for header and border to get the data row index
            Some((coords.relative_y - 2) as usize)
        } else {
            None
        }
    }

    fn detect_button_click(&self, coords: &TableCoords, row_index: usize) -> Option<UiAction> {
        // Check if clicking in the actions column (first 9 characters)
        if coords.relative_x < 9 {
            // Actions column clicked
            // Button layout: "[▶] [📋]" (positions 0-8)
            // [▶] = positions 0-2
            // space = position 3
            // [📋] = positions 4-7
            if coords.relative_x < 3 {
                // Resume button clicked
                Some(UiAction::Session(SessionAction::Resume(row_index)))
            } else if (4..8).contains(&coords.relative_x) {
                // Copy button clicked
                Some(UiAction::Session(SessionAction::Copy(row_index)))
            } else {
                None
            }
        } else {
            // If clicking elsewhere on the row, just select it (no additional action beyond selection)
            // Selection will be handled by the caller
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
}
