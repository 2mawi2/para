use crate::ui::monitor::state::MonitorAppState;
use crate::ui::monitor::AppMode;
use ratatui::{
    layout::Alignment,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::components::{
    create_control_buttons_line, create_styled_span, DialogBuilder, COLOR_BLUE, COLOR_GRAY,
    COLOR_RED, COLOR_WHITE,
};

pub struct DialogRenderer;

impl DialogRenderer {
    pub fn new() -> Self {
        Self
    }

    /// Render the finish prompt dialog for entering commit messages
    pub fn render_finish_prompt(&self, f: &mut Frame, state: &MonitorAppState) {
        let dialog = DialogBuilder::new(" Finish Session ")
            .size(60, 25)
            .border_color(COLOR_BLUE);

        let area = dialog.render_area(f);

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
        .block(dialog.create_block())
        .style(dialog.create_style());

        f.render_widget(prompt, area);
    }

    /// Render the cancel confirmation dialog
    pub fn render_cancel_confirm(&self, f: &mut Frame) {
        let dialog = DialogBuilder::new(" Confirm Cancel ")
            .size(50, 20)
            .border_color(COLOR_RED);

        let area = dialog.render_area(f);

        let confirm = Paragraph::new(vec![
            Line::from("Cancel this session?"),
            Line::from(""),
            create_control_buttons_line("confirm", "cancel"),
        ])
        .block(dialog.create_block())
        .style(dialog.create_style())
        .alignment(Alignment::Center);

        f.render_widget(confirm, area);
    }

    /// Render error dialog with customizable error message
    pub fn render_error_dialog(&self, f: &mut Frame, state: &MonitorAppState) {
        let dialog = DialogBuilder::new(" Error ")
            .size(60, 25)
            .border_color(COLOR_RED);

        let area = dialog.render_area(f);

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
                Span::styled(
                    "[Enter]",
                    Style::default().fg(super::components::COLOR_GREEN),
                ),
                Span::raw(" or "),
                Span::styled("[Esc]", Style::default().fg(super::components::COLOR_GREEN)),
                Span::raw(" to dismiss"),
            ]),
        ])
        .block(dialog.create_block())
        .style(dialog.create_style())
        .alignment(Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(error_popup, area);
    }

    /// Render appropriate dialog based on the current app mode
    pub fn render_dialog(&self, f: &mut Frame, state: &MonitorAppState) {
        match state.mode {
            AppMode::FinishPrompt => self.render_finish_prompt(f, state),
            AppMode::CancelConfirm => self.render_cancel_confirm(f),
            AppMode::ErrorDialog => self.render_error_dialog(f, state),
            _ => {} // No dialog to render
        }
    }
}

impl Default for DialogRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_renderer_creation() {
        let renderer = DialogRenderer::new();
        // Just test that we can create the renderer instance
        assert_eq!(std::mem::size_of_val(&renderer), 0); // Zero-sized struct
    }

    #[test]
    fn test_dialog_renderer_default() {
        let renderer = DialogRenderer;
        assert_eq!(std::mem::size_of_val(&renderer), 0);
    }

    // Note: Actual rendering tests would require a terminal backend
    // These tests verify the structure exists and can be instantiated
}
