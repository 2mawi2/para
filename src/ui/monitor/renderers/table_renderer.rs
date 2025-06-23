use crate::ui::monitor::state::{ButtonClick, MonitorAppState};
use crate::ui::monitor::{format_activity, truncate_task, SessionInfo};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use super::components::{
    COLOR_BLUE, COLOR_BORDER, COLOR_GRAY, COLOR_GREEN, COLOR_LIGHT_GRAY, COLOR_NORMAL_TEXT,
    COLOR_SELECTED_BG, COLOR_WHITE,
};
use super::status_renderer::StatusRenderer;

pub struct TableRenderer {
    status_renderer: StatusRenderer,
}

impl TableRenderer {
    pub fn new() -> Self {
        Self {
            status_renderer: StatusRenderer::new(),
        }
    }

    /// Render the main sessions table
    pub fn render_table(
        &self,
        f: &mut Frame,
        area: Rect,
        sessions: &[SessionInfo],
        state: &mut MonitorAppState,
    ) {
        let header = self.create_table_header();
        let rows = self.create_table_rows(sessions, state);
        let table = self.create_table_widget(rows, header);

        // Store the table area for mouse click handling
        state.set_table_area(area);

        f.render_stateful_widget(table, area, &mut state.table_state.clone());
    }

    /// Create the table header row
    fn create_table_header(&self) -> Row {
        Row::new(vec![
            Cell::from("Actions"),
            Cell::from("Session"),
            Cell::from("State"),
            Cell::from("Last Modified"),
            Cell::from("Current Task"),
            Cell::from("Tests"),
            Cell::from("Progress"),
            Cell::from("Confidence"),
            Cell::from("Changes"),
        ])
        .style(
            Style::default()
                .fg(COLOR_LIGHT_GRAY)
                .add_modifier(Modifier::BOLD),
        )
        .height(1)
    }

    /// Create all table rows from session data
    fn create_table_rows<'a>(
        &'a self,
        sessions: &'a [SessionInfo],
        state: &MonitorAppState,
    ) -> Vec<Row<'a>> {
        sessions
            .iter()
            .enumerate()
            .map(|(i, session)| self.create_session_row(session, i, state))
            .collect()
    }

    /// Create a single session row with all its cells
    fn create_session_row<'a>(
        &'a self,
        session: &'a SessionInfo,
        index: usize,
        state: &MonitorAppState,
    ) -> Row<'a> {
        let is_selected = index == state.selected_index;
        let is_stale = session.status.should_dim();
        let base_style = self.get_base_row_style(is_selected, is_stale);

        Row::new(vec![
            self.create_action_buttons_cell(is_selected, index, state),
            Cell::from(session.name.clone()).style(base_style.add_modifier(Modifier::BOLD)),
            self.create_state_cell(session, is_stale),
            Cell::from(format_activity(&session.last_activity)).style(base_style),
            Cell::from(truncate_task(&session.task, 40)).style(base_style),
            self.status_renderer
                .create_test_cell(&session.test_status, is_stale),
            self.status_renderer
                .create_progress_cell(session.todo_percentage, is_stale),
            self.status_renderer
                .create_confidence_cell(&session.confidence, is_stale),
            self.status_renderer
                .create_diff_stats_cell(&session.diff_stats, is_stale),
        ])
        .height(1)
    }

    /// Create action buttons cell for resume and copy operations
    fn create_action_buttons_cell<'a>(
        &'a self,
        is_selected: bool,
        index: usize,
        state: &MonitorAppState,
    ) -> Cell<'a> {
        // Check if any button is clicked for this row
        let button_click = state.get_button_click();
        let resume_clicked = matches!(button_click, Some(ButtonClick::Resume(i)) if *i == index);
        let copy_clicked = matches!(button_click, Some(ButtonClick::Copy(i)) if *i == index);

        // Use box-drawing characters to create button appearance
        let buttons = if is_selected {
            // Show buttons with highlighting when row is selected
            Line::from(vec![
                Span::styled(
                    if resume_clicked { "[✓]" } else { "[▶]" },
                    Style::default()
                        .fg(if resume_clicked {
                            COLOR_WHITE
                        } else {
                            COLOR_GREEN
                        })
                        .bg(if resume_clicked {
                            COLOR_GREEN
                        } else {
                            COLOR_SELECTED_BG
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    if copy_clicked { "[✓]" } else { "[📋]" },
                    Style::default()
                        .fg(if copy_clicked {
                            COLOR_WHITE
                        } else {
                            COLOR_BLUE
                        })
                        .bg(if copy_clicked {
                            COLOR_BLUE
                        } else {
                            COLOR_SELECTED_BG
                        })
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        } else {
            // Show dimmed buttons with borders when not selected, but still show click feedback
            Line::from(vec![
                if resume_clicked {
                    Span::styled("[✓]", Style::default().fg(COLOR_WHITE).bg(COLOR_GREEN))
                } else {
                    Span::styled("[▶]", Style::default().fg(COLOR_GRAY))
                },
                Span::raw(" "),
                if copy_clicked {
                    Span::styled("[✓]", Style::default().fg(COLOR_WHITE).bg(COLOR_BLUE))
                } else {
                    Span::styled("[📋]", Style::default().fg(COLOR_GRAY))
                },
            ])
        };

        Cell::from(buttons)
    }

    /// Get the base styling for a table row
    fn get_base_row_style(&self, is_selected: bool, is_stale: bool) -> Style {
        if is_selected {
            Style::default().bg(COLOR_SELECTED_BG).fg(COLOR_WHITE)
        } else if is_stale {
            Style::default().fg(crate::ui::monitor::types::SessionStatus::dimmed_text_color())
        } else {
            Style::default().fg(COLOR_NORMAL_TEXT)
        }
    }

    /// Create a session state cell with appropriate styling
    fn create_state_cell<'a>(&'a self, session: &'a SessionInfo, _is_stale: bool) -> Cell<'a> {
        let state_text = if session.is_blocked {
            "Blocked"
        } else {
            session.status.name()
        };

        let state_style = if session.is_blocked {
            Style::default().fg(super::components::COLOR_RED)
        } else {
            Style::default().fg(session.status.color())
        };

        Cell::from(state_text).style(state_style)
    }

    /// Create the table widget with proper styling and constraints
    fn create_table_widget<'a>(&self, rows: Vec<Row<'a>>, header: Row<'a>) -> Table<'a> {
        Table::new(
            rows,
            [
                Constraint::Length(9),  // Actions column
                Constraint::Min(20),    // Session name
                Constraint::Length(10), // State
                Constraint::Length(14), // Last Modified
                Constraint::Min(30),    // Current Task
                Constraint::Length(10), // Tests
                Constraint::Length(13), // Progress
                Constraint::Length(10), // Confidence
                Constraint::Length(12), // Changes
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_style(Style::default().fg(COLOR_BORDER)),
        )
    }
}

impl Default for TableRenderer {
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
                last_activity: Utc::now(),
                task: "Task 2".to_string(),
                worktree_path: PathBuf::from("/tmp/session2"),
                test_status: None,
                confidence: None,
                diff_stats: None,
                todo_percentage: None,
                is_blocked: false,
            },
        ]
    }

    #[test]
    fn test_table_renderer_creation() {
        let _renderer = TableRenderer::new();
        // Just test that we can create the renderer instance
        assert_eq!(std::mem::size_of::<StatusRenderer>(), 0); // Should be zero-sized
    }

    #[test]
    fn test_get_base_row_style() {
        let renderer = TableRenderer::new();

        // Test selected style
        let selected_style = renderer.get_base_row_style(true, false);
        assert_eq!(selected_style.bg, Some(COLOR_SELECTED_BG));
        assert_eq!(selected_style.fg, Some(COLOR_WHITE));

        // Test stale style
        let stale_style = renderer.get_base_row_style(false, true);
        assert_eq!(
            stale_style.fg,
            Some(crate::ui::monitor::types::SessionStatus::dimmed_text_color())
        );

        // Test normal style
        let normal_style = renderer.get_base_row_style(false, false);
        assert_eq!(normal_style.fg, Some(COLOR_NORMAL_TEXT));
    }

    #[test]
    fn test_create_table_rows() {
        let renderer = TableRenderer::new();
        let sessions = create_test_sessions();
        let state = MonitorAppState::new();

        let rows = renderer.create_table_rows(&sessions, &state);
        assert_eq!(rows.len(), 2);
    }

    // Note: Actual rendering tests would require a terminal backend
    // These tests verify the structure exists and can be instantiated
}
