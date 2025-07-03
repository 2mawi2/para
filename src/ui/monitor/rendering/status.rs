use super::helpers::*;
use crate::config::Config;
use crate::core::session::SessionManager;
use crate::ui::monitor::state::MonitorAppState;
use crate::ui::monitor::SessionInfo;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::path::PathBuf;

pub fn render_header(f: &mut Frame, area: Rect) {
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

pub fn render_footer(
    f: &mut Frame,
    area: Rect,
    sessions: &[SessionInfo],
    state: &MonitorAppState,
    config: &Config,
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
    let session_manager = SessionManager::new(config);
    let is_current_session =
        if let Ok(Some(current_session)) = session_manager.find_session_by_path(&current_dir) {
            current_session.name == selected_session
        } else {
            false
        };

    let session_info = if is_current_session {
        format!("{selected_session} • {selected_branch} • (CURRENT) • ")
    } else {
        format!("{selected_session} • {selected_branch} • ")
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

pub fn render_feedback_message(f: &mut Frame, state: &MonitorAppState) {
    if let Some(message) = state.get_feedback_message() {
        let area = f.area();

        // Create a more compact toast notification
        let icon = if message.contains("Copied") {
            "✓ "
        } else {
            "• "
        };
        let toast_text = format!("{icon}{message}");

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
