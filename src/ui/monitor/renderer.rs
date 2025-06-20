use crate::config::Config;
use crate::core::session::SessionManager;
use crate::ui::monitor::state::MonitorAppState;
use crate::ui::monitor::{centered_rect, format_activity, truncate_task, AppMode, SessionInfo};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
    Frame,
};
use std::path::PathBuf;

/// Helper structure to encapsulate row styling context
#[allow(dead_code)]
struct RowContext {
    pub is_selected: bool,
    pub is_stale: bool,
    pub base_style: Style,
}

impl RowContext {
    fn new(session: &SessionInfo, index: usize, state: &MonitorAppState) -> Self {
        let is_selected = index == state.selected_index;
        let is_stale = session.status.should_dim();
        
        let base_style = if is_selected {
            Style::default()
                .bg(Color::Rgb(30, 41, 59))
                .fg(Color::Rgb(255, 255, 255))
        } else if is_stale {
            Style::default()
                .fg(crate::ui::monitor::types::SessionStatus::dimmed_text_color())
        } else {
            Style::default().fg(Color::Rgb(229, 231, 235))
        };
        
        Self {
            is_selected,
            is_stale,
            base_style,
        }
    }
    
    fn default_color(&self) -> Color {
        if self.is_stale {
            crate::ui::monitor::types::SessionStatus::dimmed_text_color()
        } else {
            Color::Rgb(107, 114, 128)
        }
    }
}

fn create_progress_bar(percentage: u8) -> String {
    const BAR_WIDTH: usize = 8;
    let filled = (percentage as f32 / 100.0 * BAR_WIDTH as f32).round() as usize;
    let filled = filled.min(BAR_WIDTH);

    let mut bar = String::with_capacity(BAR_WIDTH + 5);

    for _ in 0..filled {
        bar.push('█');
    }

    for _ in filled..BAR_WIDTH {
        bar.push('░');
    }
    bar.push(' ');
    bar.push_str(&format!("{}%", percentage));

    bar
}

pub struct MonitorRenderer {
    config: Config,
}

impl MonitorRenderer {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn render(&self, f: &mut Frame, sessions: &[SessionInfo], state: &MonitorAppState) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .margin(1)
            .split(f.area());

        self.render_header(f, main_layout[0]);
        self.render_table(f, main_layout[1], sessions, state);
        self.render_footer(f, main_layout[2], sessions, state);
        match state.mode {
            AppMode::FinishPrompt => self.render_finish_prompt(f, state),
            AppMode::CancelConfirm => self.render_cancel_confirm(f),
            AppMode::ErrorDialog => self.render_error_dialog(f, state),
            _ => {}
        }
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let header_text = vec![
            Line::from(vec![
                Span::styled(
                    "Para Monitor - Interactive Session Control",
                    Style::default()
                        .fg(Color::Rgb(255, 255, 255))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("                  "),
                Span::styled(
                    "Auto-refresh: 2s",
                    Style::default().fg(Color::Rgb(156, 163, 175)),
                ),
            ]),
            Line::from("─".repeat(area.width as usize)),
        ];

        let header = Paragraph::new(header_text)
            .style(Style::default().fg(Color::Rgb(75, 85, 99)))
            .alignment(Alignment::Left);

        f.render_widget(header, area);
    }

    fn render_table(
        &self,
        f: &mut Frame,
        area: Rect,
        sessions: &[SessionInfo],
        state: &MonitorAppState,
    ) {
        let header = self.create_table_header();
        let rows = self.create_table_rows(sessions, state);
        let table = self.create_table_widget(rows, header);
        
        f.render_stateful_widget(table, area, &mut state.table_state.clone());
    }

    fn create_table_header(&self) -> Row {
        Row::new(vec![
            Cell::from("Session"),
            Cell::from("State"),
            Cell::from("Last Modified"),
            Cell::from("Current Task"),
            Cell::from("Tests"),
            Cell::from("Progress"),
            Cell::from("Confidence"),
        ])
        .style(
            Style::default()
                .fg(Color::Rgb(156, 163, 175))
                .add_modifier(Modifier::BOLD),
        )
        .height(1)
    }

    fn create_table_rows<'a>(&self, sessions: &'a [SessionInfo], state: &MonitorAppState) -> Vec<Row<'a>> {
        sessions
            .iter()
            .enumerate()
            .map(|(i, session)| self.create_session_row(session, i, state))
            .collect()
    }

    fn create_session_row<'a>(&self, session: &'a SessionInfo, index: usize, state: &MonitorAppState) -> Row<'a> {
        let row_context = RowContext::new(session, index, state);
        
        Row::new(vec![
            self.create_session_name_cell(session, &row_context),
            self.create_state_cell(session, &row_context),
            self.create_activity_cell(session, &row_context),
            self.create_task_cell(session, &row_context),
            self.create_test_cell(session, &row_context),
            self.create_progress_cell(session, &row_context),
            self.create_confidence_cell(session, &row_context),
        ])
        .height(1)
    }

    fn create_session_name_cell<'a>(&self, session: &'a SessionInfo, context: &RowContext) -> Cell<'a> {
        Cell::from(session.name.clone())
            .style(context.base_style.add_modifier(Modifier::BOLD))
    }

    fn create_state_cell<'a>(&self, session: &'a SessionInfo, _context: &RowContext) -> Cell<'a> {
        let state_text = if session.is_blocked {
            "Blocked"
        } else {
            session.status.name()
        };

        let state_style = if session.is_blocked {
            Style::default().fg(Color::Rgb(239, 68, 68))
        } else {
            Style::default().fg(session.status.color())
        };

        Cell::from(state_text).style(state_style)
    }

    fn create_activity_cell<'a>(&self, session: &'a SessionInfo, context: &RowContext) -> Cell<'a> {
        let activity_text = format_activity(&session.last_activity);
        Cell::from(activity_text).style(context.base_style)
    }

    fn create_task_cell<'a>(&self, session: &'a SessionInfo, context: &RowContext) -> Cell<'a> {
        Cell::from(truncate_task(&session.task, 40)).style(context.base_style)
    }

    fn create_test_cell<'a>(&self, session: &'a SessionInfo, context: &RowContext) -> Cell<'a> {
        match &session.test_status {
            Some(test_status) => {
                let (text, color) = self.get_test_status_display(test_status, context.is_stale);
                Cell::from(text).style(Style::default().fg(color))
            }
            None => Cell::from("-").style(Style::default().fg(context.default_color()))
        }
    }

    fn create_progress_cell<'a>(&self, session: &'a SessionInfo, context: &RowContext) -> Cell<'a> {
        match session.todo_percentage {
            Some(pct) => {
                let progress_bar = create_progress_bar(pct);
                let color = self.get_progress_color(pct, context.is_stale);
                Cell::from(progress_bar).style(Style::default().fg(color))
            }
            None => {
                let empty_bar = "░░░░░░░░ ─";
                Cell::from(empty_bar).style(Style::default().fg(context.default_color()))
            }
        }
    }

    fn create_confidence_cell<'a>(&self, session: &'a SessionInfo, context: &RowContext) -> Cell<'a> {
        match &session.confidence {
            Some(confidence) => {
                let (text, color) = self.get_confidence_display(confidence, context.is_stale);
                Cell::from(text).style(Style::default().fg(color))
            }
            None => Cell::from("-").style(Style::default().fg(context.default_color()))
        }
    }

    fn create_table_widget<'a>(&self, rows: Vec<Row<'a>>, header: Row<'a>) -> Table<'a> {
        Table::new(
            rows,
            [
                Constraint::Min(20),
                Constraint::Length(10),
                Constraint::Length(14),
                Constraint::Min(30),
                Constraint::Length(10),
                Constraint::Length(13),
                Constraint::Length(10),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_style(Style::default().fg(Color::Rgb(75, 85, 99))),
        )
    }

    fn get_test_status_display(&self, test_status: &crate::core::status::TestStatus, is_stale: bool) -> (&'static str, Color) {
        use crate::core::status::TestStatus;
        use crate::ui::monitor::types::SessionStatus;
        
        let (text, normal_color) = match test_status {
            TestStatus::Passed => ("Passed", Color::Rgb(34, 197, 94)),
            TestStatus::Failed => ("Failed", Color::Rgb(239, 68, 68)),
            TestStatus::Unknown => ("Unknown", Color::Rgb(156, 163, 175)),
        };
        
        let color = if is_stale {
            SessionStatus::dimmed_text_color()
        } else {
            normal_color
        };
        
        (text, color)
    }

    fn get_progress_color(&self, percentage: u8, is_stale: bool) -> Color {
        if is_stale {
            crate::ui::monitor::types::SessionStatus::dimmed_text_color()
        } else if percentage == 100 {
            Color::Rgb(34, 197, 94)
        } else if percentage >= 50 {
            Color::Rgb(99, 102, 241)
        } else {
            Color::Rgb(245, 158, 11)
        }
    }

    fn get_confidence_display(&self, confidence: &crate::core::status::ConfidenceLevel, is_stale: bool) -> (&'static str, Color) {
        use crate::core::status::ConfidenceLevel;
        use crate::ui::monitor::types::SessionStatus;
        
        let (text, normal_color) = match confidence {
            ConfidenceLevel::High => ("High", Color::Rgb(34, 197, 94)),
            ConfidenceLevel::Medium => ("Medium", Color::Rgb(245, 158, 11)),
            ConfidenceLevel::Low => ("Low", Color::Rgb(239, 68, 68)),
        };
        
        let color = if is_stale {
            SessionStatus::dimmed_text_color()
        } else {
            normal_color
        };
        
        (text, color)
    }

    fn render_footer(
        &self,
        f: &mut Frame,
        area: Rect,
        sessions: &[SessionInfo],
        state: &MonitorAppState,
    ) {
        let selected_session = state
            .get_selected_session(sessions)
            .map(|s| s.name.as_str())
            .unwrap_or("none");

        let selected_branch = state
            .get_selected_session(sessions)
            .map(|s| s.branch.as_str())
            .unwrap_or("");

        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let session_manager = SessionManager::new(&self.config);
        let is_current_session =
            if let Ok(Some(current_session)) = session_manager.find_session_by_path(&current_dir) {
                current_session.name == selected_session
            } else {
                false
            };

        let session_info = if is_current_session {
            format!("{} • {} • (CURRENT) • ", selected_session, selected_branch)
        } else {
            format!("{} • {} • ", selected_session, selected_branch)
        };
        let controls = vec![Line::from(vec![
            Span::styled(session_info, Style::default().fg(Color::Rgb(156, 163, 175))),
            Span::styled(
                "[Enter]",
                Style::default()
                    .fg(Color::Rgb(99, 102, 241))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Resume • "),
            Span::styled(
                "[f]",
                Style::default()
                    .fg(Color::Rgb(99, 102, 241))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Finish • "),
            Span::styled(
                "[c]",
                Style::default()
                    .fg(Color::Rgb(99, 102, 241))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Cancel • "),
            Span::styled(
                "[q]",
                Style::default()
                    .fg(Color::Rgb(99, 102, 241))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Quit"),
        ])];

        let footer = Paragraph::new(controls)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::Rgb(75, 85, 99))),
            )
            .alignment(Alignment::Left);

        f.render_widget(footer, area);
    }

    fn render_finish_prompt(&self, f: &mut Frame, state: &MonitorAppState) {
        let area = centered_rect(60, 25, f.area());

        f.render_widget(Clear, area);

        let input_text = if state.get_input().is_empty() {
            "Type your commit message..."
        } else {
            state.get_input()
        };

        let prompt = Paragraph::new(vec![
            Line::from("Enter commit message:"),
            Line::from(""),
            Line::from(Span::styled(
                input_text,
                if state.get_input().is_empty() {
                    Style::default().fg(Color::Rgb(107, 114, 128))
                } else {
                    Style::default().fg(Color::Rgb(255, 255, 255))
                },
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("[Enter]", Style::default().fg(Color::Rgb(34, 197, 94))),
                Span::raw(" confirm • "),
                Span::styled("[Esc]", Style::default().fg(Color::Rgb(239, 68, 68))),
                Span::raw(" cancel"),
            ]),
        ])
        .block(
            Block::default()
                .title(" Finish Session ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(99, 102, 241)))
                .style(Style::default().bg(Color::Rgb(0, 0, 0))),
        )
        .style(Style::default().fg(Color::Rgb(255, 255, 255)));

        f.render_widget(prompt, area);
    }

    fn render_cancel_confirm(&self, f: &mut Frame) {
        let area = centered_rect(50, 20, f.area());

        f.render_widget(Clear, area);

        let confirm = Paragraph::new(vec![
            Line::from("Cancel this session?"),
            Line::from(""),
            Line::from(vec![
                Span::styled("[Enter]", Style::default().fg(Color::Rgb(34, 197, 94))),
                Span::raw(" confirm • "),
                Span::styled("[Esc]", Style::default().fg(Color::Rgb(239, 68, 68))),
                Span::raw(" cancel"),
            ]),
        ])
        .block(
            Block::default()
                .title(" Confirm Cancel ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(239, 68, 68)))
                .style(Style::default().bg(Color::Rgb(0, 0, 0))),
        )
        .style(Style::default().fg(Color::Rgb(255, 255, 255)))
        .alignment(Alignment::Center);

        f.render_widget(confirm, area);
    }

    fn render_error_dialog(&self, f: &mut Frame, state: &MonitorAppState) {
        let area = centered_rect(60, 25, f.area());

        f.render_widget(Clear, area);

        let error_message = state.error_message.as_deref().unwrap_or("Unknown error");

        let error_popup = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("⚠️  ", Style::default().fg(Color::Rgb(239, 68, 68))),
                Span::styled(
                    "Error",
                    Style::default()
                        .fg(Color::Rgb(239, 68, 68))
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::raw(error_message)),
            Line::from(""),
            Line::from(vec![
                Span::styled("[Enter]", Style::default().fg(Color::Rgb(34, 197, 94))),
                Span::raw(" or "),
                Span::styled("[Esc]", Style::default().fg(Color::Rgb(34, 197, 94))),
                Span::raw(" to dismiss"),
            ]),
        ])
        .block(
            Block::default()
                .title(" Error ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(239, 68, 68)))
                .style(Style::default().bg(Color::Rgb(0, 0, 0))),
        )
        .style(Style::default().fg(Color::Rgb(255, 255, 255)))
        .alignment(Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(error_popup, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::monitor::SessionStatus;
    use chrono::Utc;
    use std::path::PathBuf;

    fn create_test_config() -> Config {
        // Create a minimal test config without using test_utils
        Config {
            ide: crate::config::IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: crate::config::WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: crate::config::DirectoryConfig {
                subtrees_dir: "/tmp/subtrees".to_string(),
                state_dir: "/tmp/.para_state".to_string(),
            },
            git: crate::config::GitConfig {
                branch_prefix: "para".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: crate::config::SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

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
                todo_percentage: None,
                is_blocked: false,
            },
        ]
    }

    #[test]
    fn test_renderer_creation() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);

        // Just test that we can create the renderer instance
        assert_eq!(renderer.config.git.branch_prefix, "para");
    }

    #[test]
    fn test_render_components() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);
        let sessions = create_test_sessions();
        let state = MonitorAppState::new();

        // Test that rendering methods exist and can be called
        // We can't easily test the actual rendering without a terminal backend,
        // but we can verify the structure exists
        assert_eq!(sessions.len(), 2);
        assert_eq!(state.mode, AppMode::Normal);
        assert_eq!(renderer.config.git.branch_prefix, "para");
    }

    #[test]
    fn test_create_progress_bar() {
        // Test empty progress (0%)
        assert_eq!(create_progress_bar(0), "░░░░░░░░ 0%");

        // Test partial progress (25%)
        assert_eq!(create_progress_bar(25), "██░░░░░░ 25%");

        // Test half progress (50%)
        assert_eq!(create_progress_bar(50), "████░░░░ 50%");

        // Test mostly complete (75%)
        assert_eq!(create_progress_bar(75), "██████░░ 75%");

        // Test complete (100%)
        assert_eq!(create_progress_bar(100), "████████ 100%");

        // Test edge cases
        assert_eq!(create_progress_bar(1), "░░░░░░░░ 1%"); // Very small progress rounds to 0 blocks
        assert_eq!(create_progress_bar(13), "█░░░░░░░ 13%"); // 13% = 1.04 blocks ≈ 1 block
        assert_eq!(create_progress_bar(99), "████████ 99%"); // Almost complete rounds to full blocks
    }
}
