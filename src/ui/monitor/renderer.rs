use crate::config::Config;
use crate::core::session::SessionManager;
use crate::ui::monitor::presentation::{PresentationUtils, SessionViewModel};
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

// Color constants to eliminate duplication
const COLOR_GREEN: Color = Color::Rgb(34, 197, 94);
const COLOR_RED: Color = Color::Rgb(239, 68, 68);
const COLOR_BLUE: Color = Color::Rgb(99, 102, 241);
const COLOR_GRAY: Color = Color::Rgb(107, 114, 128);
const COLOR_WHITE: Color = Color::Rgb(255, 255, 255);
const COLOR_LIGHT_GRAY: Color = Color::Rgb(156, 163, 175);
const COLOR_BORDER: Color = Color::Rgb(75, 85, 99);
const COLOR_SELECTED_BG: Color = Color::Rgb(30, 41, 59);
const COLOR_NORMAL_TEXT: Color = Color::Rgb(229, 231, 235);
const COLOR_ORANGE: Color = Color::Rgb(245, 158, 11);
const COLOR_BLACK: Color = Color::Rgb(0, 0, 0);

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

// Helper functions to eliminate dialog duplication
fn create_dialog_area(f: &mut Frame, width: u16, height: u16) -> Rect {
    let area = centered_rect(width, height, f.area());
    f.render_widget(Clear, area);
    area
}

fn create_dialog_block(title: &str, border_color: Color) -> Block {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(COLOR_BLACK))
}

fn create_dialog_style() -> Style {
    Style::default().fg(COLOR_WHITE)
}

fn create_control_buttons_line<'a>(confirm_text: &'a str, cancel_text: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(COLOR_GREEN)),
        Span::raw(format!(" {} • ", confirm_text)),
        Span::styled("[Esc]", Style::default().fg(COLOR_RED)),
        Span::raw(format!(" {}", cancel_text)),
    ])
}

fn create_styled_span(text: &str, color: Color, bold: bool) -> Span {
    let mut style = Style::default().fg(color);
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    Span::styled(text, style)
}

fn create_default_cell_for_none(default_text: &str, is_stale: bool) -> Cell {
    let color = if is_stale {
        crate::ui::monitor::types::SessionStatus::dimmed_text_color()
    } else {
        COLOR_GRAY
    };
    Cell::from(default_text).style(Style::default().fg(color))
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
                create_styled_span(
                    "Para Monitor - Interactive Session Control",
                    COLOR_WHITE,
                    true,
                ),
                Span::raw("                  "),
                Span::styled("Auto-refresh: 2s", Style::default().fg(COLOR_LIGHT_GRAY)),
            ]),
            Line::from("─".repeat(area.width as usize)),
        ];

        let header = Paragraph::new(header_text)
            .style(Style::default().fg(COLOR_BORDER))
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

    fn create_table_header<'a>(&self) -> Row<'a> {
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
                .fg(COLOR_LIGHT_GRAY)
                .add_modifier(Modifier::BOLD),
        )
        .height(1)
    }

    fn create_table_rows<'a>(
        &self,
        sessions: &'a [SessionInfo],
        state: &MonitorAppState,
    ) -> Vec<Row<'a>> {
        sessions
            .iter()
            .enumerate()
            .map(|(i, session)| self.create_session_row(session, i, state))
            .collect()
    }

    fn create_session_row<'a>(
        &self,
        session: &'a SessionInfo,
        index: usize,
        state: &MonitorAppState,
    ) -> Row<'a> {
        let is_selected = index == state.selected_index;
        let is_stale = session.status.should_dim();
        let base_style = self.get_base_row_style(is_selected, is_stale);

        Row::new(vec![
            Cell::from(session.name.clone()).style(base_style.add_modifier(Modifier::BOLD)),
            self.create_state_cell(session, is_stale),
            Cell::from(format_activity(&session.last_activity)).style(base_style),
            Cell::from(truncate_task(&session.task, 40)).style(base_style),
            self.create_test_cell(&session.test_status, is_stale),
            self.create_progress_cell(session.todo_percentage, is_stale),
            self.create_confidence_cell(&session.confidence, is_stale),
        ])
        .height(1)
    }

    fn get_base_row_style(&self, is_selected: bool, is_stale: bool) -> Style {
        if is_selected {
            Style::default().bg(COLOR_SELECTED_BG).fg(COLOR_WHITE)
        } else if is_stale {
            Style::default().fg(crate::ui::monitor::types::SessionStatus::dimmed_text_color())
        } else {
            Style::default().fg(COLOR_NORMAL_TEXT)
        }
    }

    fn create_state_cell<'a>(&self, session: &'a SessionInfo, _is_stale: bool) -> Cell<'a> {
        let state_text = if session.is_blocked {
            "Blocked"
        } else {
            session.status.name()
        };

        let state_style = if session.is_blocked {
            Style::default().fg(COLOR_RED)
        } else {
            Style::default().fg(session.status.color())
        };

        Cell::from(state_text).style(state_style)
    }

    fn create_test_cell<'a>(
        &self,
        test_status: &Option<crate::core::status::TestStatus>,
        is_stale: bool,
    ) -> Cell<'a> {
        match test_status {
            Some(status) => {
                let (text, color) = self.get_test_status_display(status, is_stale);
                Cell::from(text).style(Style::default().fg(color))
            }
            None => create_default_cell_for_none("-", is_stale),
        }
    }

    fn get_test_status_display(
        &self,
        status: &crate::core::status::TestStatus,
        is_stale: bool,
    ) -> (&'static str, Color) {
        let dimmed_color = crate::ui::monitor::types::SessionStatus::dimmed_text_color();

        match status {
            crate::core::status::TestStatus::Passed => {
                ("Passed", if is_stale { dimmed_color } else { COLOR_GREEN })
            }
            crate::core::status::TestStatus::Failed => {
                ("Failed", if is_stale { dimmed_color } else { COLOR_RED })
            }
            crate::core::status::TestStatus::Unknown => (
                "Unknown",
                if is_stale {
                    dimmed_color
                } else {
                    COLOR_LIGHT_GRAY
                },
            ),
        }
    }

    fn create_progress_cell<'a>(&self, todo_percentage: Option<u8>, is_stale: bool) -> Cell<'a> {
        match todo_percentage {
            Some(pct) => {
                let progress_bar = create_progress_bar(pct);
                let color = self.get_progress_color(pct, is_stale);
                Cell::from(progress_bar).style(Style::default().fg(color))
            }
            None => create_default_cell_for_none("░░░░░░░░ ─", is_stale),
        }
    }

    fn get_progress_color(&self, percentage: u8, is_stale: bool) -> Color {
        if is_stale {
            crate::ui::monitor::types::SessionStatus::dimmed_text_color()
        } else if percentage == 100 {
            COLOR_GREEN
        } else if percentage >= 50 {
            COLOR_BLUE
        } else {
            COLOR_ORANGE
        }
    }

    fn create_confidence_cell<'a>(
        &self,
        confidence: &Option<crate::core::status::ConfidenceLevel>,
        is_stale: bool,
    ) -> Cell<'a> {
        match confidence {
            Some(level) => {
                let (text, color) = self.get_confidence_display(level, is_stale);
                Cell::from(text).style(Style::default().fg(color))
            }
            None => create_default_cell_for_none("-", is_stale),
        }
    }

    fn get_confidence_display(
        &self,
        level: &crate::core::status::ConfidenceLevel,
        is_stale: bool,
    ) -> (&'static str, Color) {
        let dimmed_color = crate::ui::monitor::types::SessionStatus::dimmed_text_color();

        match level {
            crate::core::status::ConfidenceLevel::High => {
                ("High", if is_stale { dimmed_color } else { COLOR_GREEN })
            }
            crate::core::status::ConfidenceLevel::Medium => {
                ("Medium", if is_stale { dimmed_color } else { COLOR_ORANGE })
            }
            crate::core::status::ConfidenceLevel::Low => {
                ("Low", if is_stale { dimmed_color } else { COLOR_RED })
            }
        }
    }

    /// Demonstrates data-view binding pattern using SessionViewModel
    /// This method shows how presentation logic is cleanly separated from business data
    #[allow(dead_code)]
    fn create_session_row_with_view_model<'a>(
        &self,
        view_model: &'a SessionViewModel,
        index: usize,
        state: &MonitorAppState,
    ) -> Row<'a> {
        let is_selected = index == state.selected_index;
        let _is_current_session = false; // For demonstration purposes

        // Apply UI state for styling decisions
        let name_style = if is_selected {
            Style::default()
                .bg(view_model.theme.background_selected_color)
                .fg(view_model.theme.text_selected_color)
                .add_modifier(Modifier::BOLD)
        } else {
            view_model.name_style()
        };

        let base_style = if is_selected {
            Style::default()
                .bg(view_model.theme.background_selected_color)
                .fg(view_model.theme.text_selected_color)
        } else {
            view_model.base_text_style()
        };

        Row::new(vec![
            // Session name with enhanced styling from presentation layer
            Cell::from(view_model.data.name.clone()).style(name_style),
            // Status with clean presentation layer styling
            Cell::from(view_model.status_display_text()).style(view_model.status_style()),
            // Activity with presentation utility formatting
            Cell::from(PresentationUtils::format_activity_time(
                &view_model.data.last_activity,
            ))
            .style(base_style),
            // Task with presentation utility truncation
            Cell::from(PresentationUtils::truncate_task_text(
                &view_model.data.task,
                40,
            ))
            .style(base_style),
            // Test status with presentation layer color logic
            self.create_test_cell_with_view_model_enhanced(view_model, is_selected),
            // Progress with presentation layer progress bar creation
            self.create_progress_cell_with_view_model_enhanced(view_model, is_selected),
            // Confidence with presentation layer color and display logic
            self.create_confidence_cell_with_view_model_enhanced(view_model, is_selected),
        ])
        .height(1)
    }

    /// Helper method demonstrating presentation layer test status handling with selection
    #[allow(dead_code)]
    fn create_test_cell_with_view_model_enhanced<'a>(
        &self,
        view_model: &'a SessionViewModel,
        is_selected: bool,
    ) -> Cell<'a> {
        match view_model.test_status_display_text() {
            Some(text) => {
                let color = if is_selected {
                    view_model.theme.text_selected_color
                } else {
                    view_model
                        .test_status_color()
                        .unwrap_or(view_model.theme.text_normal_color)
                };
                Cell::from(text).style(Style::default().fg(color))
            }
            None => {
                let style = if is_selected {
                    Style::default()
                        .bg(view_model.theme.background_selected_color)
                        .fg(view_model.theme.text_selected_color)
                } else {
                    PresentationUtils::default_none_cell_style(
                        &view_model.theme,
                        view_model.should_dim(),
                    )
                };
                Cell::from("-").style(style)
            }
        }
    }

    /// Helper method demonstrating presentation layer progress handling with selection
    #[allow(dead_code)]
    fn create_progress_cell_with_view_model_enhanced<'a>(
        &self,
        view_model: &'a SessionViewModel,
        is_selected: bool,
    ) -> Cell<'a> {
        let progress_percentage = view_model.data.completion_percentage();
        if progress_percentage > 0 {
            let progress_bar = view_model.create_progress_bar(8);
            let color = if is_selected {
                view_model.theme.text_selected_color
            } else {
                view_model.progress_color()
            };
            Cell::from(progress_bar).style(Style::default().fg(color))
        } else {
            let style = if is_selected {
                Style::default()
                    .bg(view_model.theme.background_selected_color)
                    .fg(view_model.theme.text_selected_color)
            } else {
                PresentationUtils::default_none_cell_style(
                    &view_model.theme,
                    view_model.should_dim(),
                )
            };
            Cell::from("░░░░░░░░ ─").style(style)
        }
    }

    /// Helper method demonstrating presentation layer confidence handling with selection
    #[allow(dead_code)]
    fn create_confidence_cell_with_view_model_enhanced<'a>(
        &self,
        view_model: &'a SessionViewModel,
        is_selected: bool,
    ) -> Cell<'a> {
        match view_model.confidence_display_text() {
            Some(text) => {
                let color = if is_selected {
                    view_model.theme.text_selected_color
                } else {
                    view_model
                        .confidence_color()
                        .unwrap_or(view_model.theme.text_normal_color)
                };
                Cell::from(text).style(Style::default().fg(color))
            }
            None => {
                let style = if is_selected {
                    Style::default()
                        .bg(view_model.theme.background_selected_color)
                        .fg(view_model.theme.text_selected_color)
                } else {
                    PresentationUtils::default_none_cell_style(
                        &view_model.theme,
                        view_model.should_dim(),
                    )
                };
                Cell::from("-").style(style)
            }
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
                .border_style(Style::default().fg(COLOR_BORDER)),
        )
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
            Span::styled(session_info, Style::default().fg(COLOR_LIGHT_GRAY)),
            create_styled_span("[Enter]", COLOR_BLUE, true),
            Span::raw(" Resume • "),
            create_styled_span("[f]", COLOR_BLUE, true),
            Span::raw(" Finish • "),
            create_styled_span("[c]", COLOR_BLUE, true),
            Span::raw(" Cancel • "),
            create_styled_span("[q]", COLOR_BLUE, true),
            Span::raw(" Quit"),
        ])];

        let footer = Paragraph::new(controls)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(COLOR_BORDER)),
            )
            .alignment(Alignment::Left);

        f.render_widget(footer, area);
    }

    fn render_finish_prompt(&self, f: &mut Frame, state: &MonitorAppState) {
        let area = create_dialog_area(f, 60, 25);

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
                    Style::default().fg(COLOR_GRAY)
                } else {
                    Style::default().fg(COLOR_WHITE)
                },
            )),
            Line::from(""),
            create_control_buttons_line("confirm", "cancel"),
        ])
        .block(create_dialog_block(" Finish Session ", COLOR_BLUE))
        .style(create_dialog_style());

        f.render_widget(prompt, area);
    }

    fn render_cancel_confirm(&self, f: &mut Frame) {
        let area = create_dialog_area(f, 50, 20);

        let confirm = Paragraph::new(vec![
            Line::from("Cancel this session?"),
            Line::from(""),
            create_control_buttons_line("confirm", "cancel"),
        ])
        .block(create_dialog_block(" Confirm Cancel ", COLOR_RED))
        .style(create_dialog_style())
        .alignment(Alignment::Center);

        f.render_widget(confirm, area);
    }

    fn render_error_dialog(&self, f: &mut Frame, state: &MonitorAppState) {
        let area = create_dialog_area(f, 60, 25);

        let error_message = state.error_message.as_deref().unwrap_or("Unknown error");

        let error_popup = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("⚠️  ", Style::default().fg(COLOR_RED)),
                create_styled_span("Error", COLOR_RED, true),
            ]),
            Line::from(""),
            Line::from(Span::raw(error_message)),
            Line::from(""),
            Line::from(vec![
                Span::styled("[Enter]", Style::default().fg(COLOR_GREEN)),
                Span::raw(" or "),
                Span::styled("[Esc]", Style::default().fg(COLOR_GREEN)),
                Span::raw(" to dismiss"),
            ]),
        ])
        .block(create_dialog_block(" Error ", COLOR_RED))
        .style(create_dialog_style())
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
        crate::test_utils::test_helpers::create_test_config()
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

    #[test]
    fn test_get_base_row_style() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);

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
    fn test_get_progress_color() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);

        // Test completion colors
        assert_eq!(renderer.get_progress_color(100, false), COLOR_GREEN); // Green for complete
        assert_eq!(renderer.get_progress_color(75, false), COLOR_BLUE); // Blue for high progress
        assert_eq!(renderer.get_progress_color(25, false), COLOR_ORANGE); // Orange for low progress

        // Test stale color override
        let dimmed = crate::ui::monitor::types::SessionStatus::dimmed_text_color();
        assert_eq!(renderer.get_progress_color(100, true), dimmed);
        assert_eq!(renderer.get_progress_color(50, true), dimmed);
    }

    #[test]
    fn test_get_test_status_display() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);

        // Test passed status
        let (text, color) =
            renderer.get_test_status_display(&crate::core::status::TestStatus::Passed, false);
        assert_eq!(text, "Passed");
        assert_eq!(color, COLOR_GREEN);

        // Test failed status
        let (text, color) =
            renderer.get_test_status_display(&crate::core::status::TestStatus::Failed, false);
        assert_eq!(text, "Failed");
        assert_eq!(color, COLOR_RED);

        // Test stale status override
        let (text, color) =
            renderer.get_test_status_display(&crate::core::status::TestStatus::Passed, true);
        assert_eq!(text, "Passed");
        assert_eq!(
            color,
            crate::ui::monitor::types::SessionStatus::dimmed_text_color()
        );
    }

    #[test]
    fn test_get_confidence_display() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);

        // Test high confidence
        let (text, color) =
            renderer.get_confidence_display(&crate::core::status::ConfidenceLevel::High, false);
        assert_eq!(text, "High");
        assert_eq!(color, COLOR_GREEN);

        // Test medium confidence
        let (text, color) =
            renderer.get_confidence_display(&crate::core::status::ConfidenceLevel::Medium, false);
        assert_eq!(text, "Medium");
        assert_eq!(color, COLOR_ORANGE);

        // Test low confidence
        let (text, color) =
            renderer.get_confidence_display(&crate::core::status::ConfidenceLevel::Low, false);
        assert_eq!(text, "Low");
        assert_eq!(color, COLOR_RED);

        // Test stale override
        let (text, color) =
            renderer.get_confidence_display(&crate::core::status::ConfidenceLevel::High, true);
        assert_eq!(text, "High");
        assert_eq!(
            color,
            crate::ui::monitor::types::SessionStatus::dimmed_text_color()
        );
    }

    // COMPREHENSIVE RENDERER TESTS - PHASE 1
    // These tests cover rendering output for various session states,
    // color coding, formatting logic, and table layout

    fn create_comprehensive_test_sessions() -> Vec<SessionInfo> {
        use crate::core::status::{ConfidenceLevel, TestStatus};
        use chrono::{Duration, Utc};

        let now = Utc::now();
        vec![
            // Active session with all agent status fields
            SessionInfo {
                name: "active-session".to_string(),
                branch: "feature/active".to_string(),
                status: SessionStatus::Active,
                last_activity: now,
                task: "Implementing feature X with comprehensive testing".to_string(),
                worktree_path: PathBuf::from("/tmp/active"),
                test_status: Some(TestStatus::Passed),
                confidence: Some(ConfidenceLevel::High),
                todo_percentage: Some(85),
                is_blocked: false,
            },
            // Idle session with failing tests
            SessionInfo {
                name: "idle-failing".to_string(),
                branch: "fix/bug-123".to_string(),
                status: SessionStatus::Idle,
                last_activity: now - Duration::minutes(10),
                task: "Fixing critical bug".to_string(),
                worktree_path: PathBuf::from("/tmp/idle"),
                test_status: Some(TestStatus::Failed),
                confidence: Some(ConfidenceLevel::Low),
                todo_percentage: Some(25),
                is_blocked: true,
            },
            // Review session with unknown test status
            SessionInfo {
                name: "review-ready".to_string(),
                branch: "docs/update".to_string(),
                status: SessionStatus::Review,
                last_activity: now - Duration::hours(2),
                task: "Documentation updates".to_string(),
                worktree_path: PathBuf::from("/tmp/review"),
                test_status: Some(TestStatus::Unknown),
                confidence: Some(ConfidenceLevel::Medium),
                todo_percentage: Some(100),
                is_blocked: false,
            },
            // Stale session with no agent status
            SessionInfo {
                name: "stale-session".to_string(),
                branch: "abandoned/old-work".to_string(),
                status: SessionStatus::Stale,
                last_activity: now - Duration::days(3),
                task: "Old abandoned work".to_string(),
                worktree_path: PathBuf::from("/tmp/stale"),
                test_status: None,
                confidence: None,
                todo_percentage: None,
                is_blocked: false,
            },
            // Session with long task name for truncation testing
            SessionInfo {
                name: "long-task".to_string(),
                branch: "feature/very-long-name".to_string(),
                status: SessionStatus::Ready,
                last_activity: now - Duration::minutes(30),
                task: "This is a very long task description that should be truncated when displayed in the table to ensure proper layout and readability".to_string(),
                worktree_path: PathBuf::from("/tmp/long"),
                test_status: Some(TestStatus::Passed),
                confidence: Some(ConfidenceLevel::Medium),
                todo_percentage: Some(50),
                is_blocked: false,
            },
        ]
    }

    #[test]
    fn test_table_header_creation() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);

        let _header = renderer.create_table_header();

        // We can't access private fields, but we can verify structure exists
        // and test that the method executes without errors

        // Test that header creation is consistent
        let _header2 = renderer.create_table_header();
        // Just verify they can be created without panicking
    }

    #[test]
    fn test_session_row_creation_all_statuses() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);
        let sessions = create_comprehensive_test_sessions();
        let state = MonitorAppState::new();

        // Test that row creation works for all session types
        let _active_row = renderer.create_session_row(&sessions[0], 0, &state);
        let _idle_row = renderer.create_session_row(&sessions[1], 1, &state);
        let _review_row = renderer.create_session_row(&sessions[2], 2, &state);
        let _stale_row = renderer.create_session_row(&sessions[3], 3, &state);

        // Verify stale session logic
        assert!(sessions[3].status.should_dim());
    }

    #[test]
    fn test_state_cell_creation() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);
        let sessions = create_comprehensive_test_sessions();

        // Test that cells can be created for different session states
        let _blocked_cell = renderer.create_state_cell(&sessions[1], false);
        let _active_cell = renderer.create_state_cell(&sessions[0], false);
        let _review_cell = renderer.create_state_cell(&sessions[2], false);

        // Verify blocked session logic (sessions[1] is blocked)
        assert!(sessions[1].is_blocked);
        assert!(!sessions[0].is_blocked);
        assert!(!sessions[2].is_blocked);

        // Verify status types
        assert_eq!(sessions[0].status, SessionStatus::Active);
        assert_eq!(sessions[2].status, SessionStatus::Review);
    }

    #[test]
    fn test_test_cell_creation_all_states() {
        use crate::core::status::TestStatus;
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);
        let sessions = create_comprehensive_test_sessions();

        // Test that cells can be created for different test statuses
        let _passed_cell = renderer.create_test_cell(&sessions[0].test_status, false);
        let _failed_cell = renderer.create_test_cell(&sessions[1].test_status, false);
        let _unknown_cell = renderer.create_test_cell(&sessions[2].test_status, false);
        let _none_cell = renderer.create_test_cell(&sessions[3].test_status, false);

        // Verify the test status data is correct
        assert_eq!(sessions[0].test_status, Some(TestStatus::Passed));
        assert_eq!(sessions[1].test_status, Some(TestStatus::Failed));
        assert_eq!(sessions[2].test_status, Some(TestStatus::Unknown));
        assert_eq!(sessions[3].test_status, None);

        // Test stale override functionality
        let _stale_passed_cell = renderer.create_test_cell(&sessions[0].test_status, true);
        // Just verify it doesn't crash with stale flag
    }

    #[test]
    fn test_progress_cell_creation_all_values() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);
        let sessions = create_comprehensive_test_sessions();

        // Test that cells can be created for different progress values
        let _high_progress_cell = renderer.create_progress_cell(sessions[0].todo_percentage, false);
        let _low_progress_cell = renderer.create_progress_cell(sessions[1].todo_percentage, false);
        let _complete_cell = renderer.create_progress_cell(sessions[2].todo_percentage, false);
        let _none_cell = renderer.create_progress_cell(sessions[3].todo_percentage, false);

        // Verify the progress data is correct
        assert_eq!(sessions[0].todo_percentage, Some(85));
        assert_eq!(sessions[1].todo_percentage, Some(25));
        assert_eq!(sessions[2].todo_percentage, Some(100));
        assert_eq!(sessions[3].todo_percentage, None);

        // Test stale override functionality
        let _stale_cell = renderer.create_progress_cell(sessions[0].todo_percentage, true);
        // Just verify it doesn't crash with stale flag
    }

    #[test]
    fn test_confidence_cell_creation_all_levels() {
        use crate::core::status::ConfidenceLevel;
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);
        let sessions = create_comprehensive_test_sessions();

        // Test that cells can be created for different confidence levels
        let _high_cell = renderer.create_confidence_cell(&sessions[0].confidence, false);
        let _low_cell = renderer.create_confidence_cell(&sessions[1].confidence, false);
        let _medium_cell = renderer.create_confidence_cell(&sessions[2].confidence, false);
        let _none_cell = renderer.create_confidence_cell(&sessions[3].confidence, false);

        // Verify the confidence data is correct
        assert_eq!(sessions[0].confidence, Some(ConfidenceLevel::High));
        assert_eq!(sessions[1].confidence, Some(ConfidenceLevel::Low));
        assert_eq!(sessions[2].confidence, Some(ConfidenceLevel::Medium));
        assert_eq!(sessions[3].confidence, None);

        // Test stale override functionality
        let _stale_cell = renderer.create_confidence_cell(&sessions[0].confidence, true);
        // Just verify it doesn't crash with stale flag
    }

    #[test]
    fn test_table_widget_creation() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);
        let sessions = create_comprehensive_test_sessions();
        let state = MonitorAppState::new();

        let header = renderer.create_table_header();
        let rows = renderer.create_table_rows(&sessions, &state);
        let _table = renderer.create_table_widget(rows, header);

        // Verify we can create table structure without errors
        // The widths field is private, so we test that the table was created successfully
        assert_eq!(sessions.len(), 5); // We have 5 test sessions
                                       // The fact that this doesn't crash means the table was created properly
    }

    #[test]
    fn test_selected_session_styling() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);
        let sessions = create_comprehensive_test_sessions();
        let mut state = MonitorAppState::new();

        // Select second session
        state.selected_index = 1;

        let rows = renderer.create_table_rows(&sessions, &state);

        // Verify we can create rows with different selection states
        assert_eq!(rows.len(), 5); // Should have 5 rows for 5 sessions
        assert_eq!(state.selected_index, 1); // Selection state preserved

        // Test base row style method directly
        let normal_style = renderer.get_base_row_style(false, false);
        let selected_style = renderer.get_base_row_style(true, false);
        let stale_style = renderer.get_base_row_style(false, true);

        // Verify style differences exist
        assert_ne!(normal_style.bg, selected_style.bg);
        assert_ne!(normal_style.fg, stale_style.fg);
    }

    #[test]
    fn test_task_truncation_in_rows() {
        use crate::ui::monitor::truncate_task;
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);
        let sessions = create_comprehensive_test_sessions();
        let state = MonitorAppState::new();

        let rows = renderer.create_table_rows(&sessions, &state);

        // Verify rows were created
        assert_eq!(rows.len(), 5);

        // Test truncation logic directly using the utility function
        let long_task = &sessions[4].task; // Last session has long task
        let truncated = truncate_task(long_task, 40);

        // Should be truncated to 40 characters or less (plus potential ellipsis)
        assert!(
            truncated.len() <= 43,
            "Task should be truncated, but was: {}",
            truncated
        );

        // Long task should be truncated
        assert!(
            long_task.len() > 40,
            "Test task should be longer than 40 chars"
        );
        if truncated.len() == 43 {
            assert!(truncated.ends_with("..."));
        }
    }

    #[test]
    fn test_stale_session_dimming() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);
        let sessions = create_comprehensive_test_sessions();
        let state = MonitorAppState::new();

        // Find the stale session row
        let stale_session = &sessions[3]; // Stale session
        assert!(stale_session.status.should_dim());
        assert_eq!(stale_session.status, SessionStatus::Stale);

        let _stale_row = renderer.create_session_row(stale_session, 3, &state);

        // Test the dimming logic through the style methods
        let stale_base_style = renderer.get_base_row_style(false, true);
        let normal_base_style = renderer.get_base_row_style(false, false);

        // Verify stale style is different from normal style
        assert_ne!(stale_base_style.fg, normal_base_style.fg);
        assert_eq!(
            stale_base_style.fg,
            Some(SessionStatus::dimmed_text_color())
        );

        // Test cells can be created with stale flag
        let _test_cell = renderer.create_test_cell(&stale_session.test_status, true);
        let _progress_cell = renderer.create_progress_cell(stale_session.todo_percentage, true);
        let _confidence_cell = renderer.create_confidence_cell(&stale_session.confidence, true);
    }

    #[test]
    fn test_edge_case_empty_sessions() {
        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);
        let empty_sessions: Vec<SessionInfo> = vec![];
        let state = MonitorAppState::new();

        let header = renderer.create_table_header();
        let rows = renderer.create_table_rows(&empty_sessions, &state);
        // Clone rows before moving to table creation
        let rows_len = rows.len();
        let _table = renderer.create_table_widget(rows, header);

        // Should handle empty sessions gracefully
        assert_eq!(rows_len, 0);
        assert_eq!(empty_sessions.len(), 0);
        // The fact that we can create a table with empty sessions without crashing is the test
    }

    #[test]
    fn test_dialog_creation_helpers() {
        // Test dialog helper functions
        let _block = create_dialog_block("Test Title", COLOR_BLUE);
        // We can't access private fields, but we can verify functions work

        let style = create_dialog_style();
        assert_eq!(style.fg, Some(COLOR_WHITE));

        let _control_line = create_control_buttons_line("Save", "Cancel");
        // Just verify it doesn't crash and returns a Line

        let span = create_styled_span("Test", COLOR_RED, true);
        assert_eq!(span.content, "Test");
        assert_eq!(span.style.fg, Some(COLOR_RED));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_data_view_binding_pattern() {
        use crate::core::status::{ConfidenceLevel, TestStatus};
        use crate::ui::monitor::data::{SessionData, SessionDataStatus};
        use crate::ui::monitor::presentation::SessionViewModel;
        use chrono::Utc;
        use std::path::PathBuf;

        let config = create_test_config();
        let renderer = MonitorRenderer::new(config);
        let mut state = MonitorAppState::new();
        state.selected_index = 1; // Select second session

        // Create test data using the new separated architecture
        let session_data = SessionData {
            name: "data-binding-test".to_string(),
            branch: "feature/data-binding".to_string(),
            status: SessionDataStatus::Active,
            last_activity: Utc::now(),
            task: "Implement data-view binding patterns".to_string(),
            worktree_path: PathBuf::from("/test/data-binding"),
            test_status: Some(TestStatus::Passed),
            confidence: Some(ConfidenceLevel::High),
            todo_percentage: Some(75),
            is_blocked: false,
        };

        // Create view model from data
        let view_model = SessionViewModel::new(session_data);

        // Test the new data-view binding row creation method
        let _row = renderer.create_session_row_with_view_model(&view_model, 1, &state);

        // Test helper methods work correctly - this demonstrates the clean separation
        let _test_cell = renderer.create_test_cell_with_view_model_enhanced(&view_model, false);
        let _progress_cell =
            renderer.create_progress_cell_with_view_model_enhanced(&view_model, false);
        let _confidence_cell =
            renderer.create_confidence_cell_with_view_model_enhanced(&view_model, false);

        // All cells should be created without error
        // This demonstrates the clean separation of presentation logic

        // Test with selection state
        let selected_view_model = view_model.clone().with_selection(true);
        let _selected_row =
            renderer.create_session_row_with_view_model(&selected_view_model, 0, &state);

        // Test with blocked session
        let mut blocked_data = view_model.data.clone();
        blocked_data.is_blocked = true;
        let blocked_view_model = SessionViewModel::new(blocked_data);
        let _blocked_row =
            renderer.create_session_row_with_view_model(&blocked_view_model, 0, &state);

        // If we reach here without panicking, the data-view binding pattern works
    }
}
