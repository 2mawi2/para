use super::helpers::*;
use crate::ui::monitor::state::MonitorAppState;
use ratatui::{
    layout::Alignment,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render_finish_prompt(f: &mut Frame, state: &MonitorAppState) {
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

pub fn render_cancel_confirm(f: &mut Frame) {
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

pub fn render_error_dialog(f: &mut Frame, state: &MonitorAppState) {
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
