use crate::config::Config;
use crate::core::session::SessionManager;
use crate::ui::monitor::state::{ButtonClick, MonitorAppState};
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

    pub fn render(&self, f: &mut Frame, sessions: &[SessionInfo], state: &mut MonitorAppState) {
        // Clear expired feedback messages and button clicks
        state.clear_expired_feedback();
        state.clear_expired_button_click();

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

        // Render feedback message if present
        if state.get_feedback_message().is_some() {
            self.render_feedback_message(f, state);
        }

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
        state: &mut MonitorAppState,
    ) {
        let header = self.create_table_header();
        let rows = self.create_table_rows(sessions, state);
        let table = self.create_table_widget(rows, header);

        // Store the table area for mouse click handling
        state.set_table_area(area);

        f.render_stateful_widget(table, area, &mut state.table_state.clone());
    }

    fn create_table_header<'a>(&self) -> Row<'a> {
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
            self.create_action_buttons_cell(is_selected, index, state),
            Cell::from(session.name.clone()).style(base_style.add_modifier(Modifier::BOLD)),
            self.create_state_cell(session, is_stale),
            Cell::from(format_activity(&session.last_activity)).style(base_style),
            Cell::from(truncate_task(&session.task, 40)).style(base_style),
            self.create_test_cell(&session.test_status, is_stale),
            self.create_progress_cell(session.todo_percentage, is_stale),
            self.create_confidence_cell(&session.confidence, is_stale),
            self.create_diff_stats_cell(&session.diff_stats, is_stale),
        ])
        .height(1)
    }

    fn create_action_buttons_cell<'a>(
        &self,
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

    fn create_diff_stats_cell<'a>(
        &self,
        diff_stats: &Option<crate::core::status::DiffStats>,
        is_stale: bool,
    ) -> Cell<'a> {
        match diff_stats {
            Some(stats) => {
                let text = format!("+{} -{}", stats.additions, stats.deletions);
                if is_stale {
                    Cell::from(text).style(
                        Style::default()
                            .fg(crate::ui::monitor::types::SessionStatus::dimmed_text_color()),
                    )
                } else {
                    // Create a colored string with green for additions and red for deletions
                    let spans = vec![
                        Span::styled(
                            format!("+{}", stats.additions),
                            Style::default().fg(COLOR_GREEN),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("-{}", stats.deletions),
                            Style::default().fg(COLOR_RED),
                        ),
                    ];
                    Cell::from(Line::from(spans))
                }
            }
            None => create_default_cell_for_none("-", is_stale),
        }
    }

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
            create_styled_span("[y]", COLOR_BLUE, true),
            Span::raw(" Copy • "),
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

    fn render_feedback_message(&self, f: &mut Frame, state: &MonitorAppState) {
        if let Some(message) = state.get_feedback_message() {
            let area = f.area();

            // Create a more compact toast notification
            let icon = if message.contains("Copied") {
                "✓ "
            } else {
                "• "
            };
            let toast_text = format!("{}{}", icon, message);

            // Calculate dimensions
            let feedback_width = (toast_text.len() as u16).min(40) + 2; // More compact
            let feedback_height = 1; // Single line toast

            // Position in bottom right corner
            let x = area.width.saturating_sub(feedback_width + 2); // 2 chars from right edge
            let y = area.height.saturating_sub(feedback_height + 2); // 2 lines from bottom

            let feedback_area = Rect {
                x,
                y,
                width: feedback_width,
                height: feedback_height,
            };

            // Create a sleek toast notification without borders
            let feedback_widget = Paragraph::new(toast_text)
                .style(
                    Style::default()
                        .fg(COLOR_BLACK)
                        .bg(COLOR_GREEN)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(Alignment::Center);

            f.render_widget(feedback_widget, feedback_area);
        }
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
}
