use super::helpers::*;
use crate::ui::monitor::state::{ButtonClick, MonitorAppState};
use crate::ui::monitor::{format_activity, truncate_task, SessionInfo};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render_table(
    f: &mut Frame,
    area: Rect,
    sessions: &[SessionInfo],
    state: &mut MonitorAppState,
) {
    let header = create_table_header();
    let rows = create_table_rows(sessions, state);
    let table = create_table_widget(rows, header);

    // Store the table area for mouse click handling
    state.set_table_area(area);

    f.render_stateful_widget(table, area, &mut state.table_state.clone());
}

fn create_table_header<'a>() -> Row<'a> {
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

fn create_table_rows<'a>(sessions: &'a [SessionInfo], state: &MonitorAppState) -> Vec<Row<'a>> {
    sessions
        .iter()
        .enumerate()
        .map(|(i, session)| create_session_row(session, i, state))
        .collect()
}

fn create_session_row<'a>(
    session: &'a SessionInfo,
    index: usize,
    state: &MonitorAppState,
) -> Row<'a> {
    let is_selected = index == state.selected_index;
    let is_stale = session.status.should_dim();
    let base_style = get_base_row_style(is_selected, is_stale);

    Row::new(vec![
        create_action_buttons_cell(is_selected, index, state),
        Cell::from(session.name.clone()).style(base_style.add_modifier(Modifier::BOLD)),
        create_state_cell(session, is_stale),
        Cell::from(format_activity(&session.last_activity)).style(base_style),
        Cell::from(truncate_task(&session.task, 40)).style(base_style),
        create_test_cell(&session.test_status, is_stale),
        create_progress_cell(session.todo_percentage, is_stale),
        create_confidence_cell(&session.confidence, is_stale),
        create_diff_stats_cell(&session.diff_stats, is_stale),
    ])
    .height(1)
}

fn create_action_buttons_cell<'a>(
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

fn get_base_row_style(is_selected: bool, is_stale: bool) -> Style {
    if is_selected {
        Style::default().bg(COLOR_SELECTED_BG).fg(COLOR_WHITE)
    } else if is_stale {
        Style::default().fg(crate::ui::monitor::types::SessionStatus::dimmed_text_color())
    } else {
        Style::default().fg(COLOR_NORMAL_TEXT)
    }
}

fn create_state_cell<'a>(session: &'a SessionInfo, _is_stale: bool) -> Cell<'a> {
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
    test_status: &Option<crate::core::status::TestStatus>,
    is_stale: bool,
) -> Cell<'a> {
    match test_status {
        Some(status) => {
            let (text, color) = get_test_status_display(status, is_stale);
            Cell::from(text).style(Style::default().fg(color))
        }
        None => create_default_cell_for_none("-", is_stale),
    }
}

fn get_test_status_display(
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

fn create_progress_cell<'a>(todo_percentage: Option<u8>, is_stale: bool) -> Cell<'a> {
    match todo_percentage {
        Some(pct) => {
            let progress_bar = create_progress_bar(pct);
            let color = get_progress_color(pct, is_stale);
            Cell::from(progress_bar).style(Style::default().fg(color))
        }
        None => create_default_cell_for_none("░░░░░░░░ ─", is_stale),
    }
}

fn get_progress_color(percentage: u8, is_stale: bool) -> Color {
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
    confidence: &Option<crate::core::status::ConfidenceLevel>,
    is_stale: bool,
) -> Cell<'a> {
    match confidence {
        Some(level) => {
            let (text, color) = get_confidence_display(level, is_stale);
            Cell::from(text).style(Style::default().fg(color))
        }
        None => create_default_cell_for_none("-", is_stale),
    }
}

fn get_confidence_display(
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

fn create_table_widget<'a>(rows: Vec<Row<'a>>, header: Row<'a>) -> Table<'a> {
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
