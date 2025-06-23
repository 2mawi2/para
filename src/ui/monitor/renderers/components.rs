use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear},
    Frame,
};

// Color constants to eliminate duplication
pub const COLOR_GREEN: Color = Color::Rgb(34, 197, 94);
pub const COLOR_RED: Color = Color::Rgb(239, 68, 68);
pub const COLOR_BLUE: Color = Color::Rgb(99, 102, 241);
pub const COLOR_GRAY: Color = Color::Rgb(107, 114, 128);
pub const COLOR_WHITE: Color = Color::Rgb(255, 255, 255);
pub const COLOR_LIGHT_GRAY: Color = Color::Rgb(156, 163, 175);
pub const COLOR_BORDER: Color = Color::Rgb(75, 85, 99);
pub const COLOR_SELECTED_BG: Color = Color::Rgb(30, 41, 59);
pub const COLOR_NORMAL_TEXT: Color = Color::Rgb(229, 231, 235);
pub const COLOR_ORANGE: Color = Color::Rgb(245, 158, 11);
pub const COLOR_BLACK: Color = Color::Rgb(0, 0, 0);

/// Builder for creating dialog components with consistent styling
pub struct DialogBuilder {
    title: String,
    width: u16,
    height: u16,
    border_color: Color,
}

impl DialogBuilder {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 60,
            height: 25,
            border_color: COLOR_BLUE,
        }
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub fn render_area(&self, f: &mut Frame) -> Rect {
        create_dialog_area(f, self.width, self.height)
    }

    pub fn create_block(&self) -> Block {
        create_dialog_block(&self.title, self.border_color)
    }

    pub fn create_style(&self) -> Style {
        create_dialog_style()
    }
}

/// Builder for creating table components with consistent styling  
#[allow(dead_code)]
pub struct TableBuilder {
    pub headers: Vec<String>,
    pub selected_bg: Color,
    pub border_color: Color,
}

impl TableBuilder {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            selected_bg: COLOR_SELECTED_BG,
            border_color: COLOR_BORDER,
        }
    }

    #[allow(dead_code)]
    pub fn headers(mut self, headers: Vec<String>) -> Self {
        self.headers = headers;
        self
    }

    #[allow(dead_code)]
    pub fn selected_bg(mut self, color: Color) -> Self {
        self.selected_bg = color;
        self
    }

    #[allow(dead_code)]
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }
}

impl Default for TableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions for dialog creation
pub fn create_dialog_area(f: &mut Frame, width: u16, height: u16) -> Rect {
    let area = crate::ui::monitor::centered_rect(width, height, f.area());
    f.render_widget(Clear, area);
    area
}

pub fn create_dialog_block(title: &str, border_color: Color) -> Block {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(COLOR_BLACK))
}

pub fn create_dialog_style() -> Style {
    Style::default().fg(COLOR_WHITE)
}

pub fn create_control_buttons_line<'a>(confirm_text: &'a str, cancel_text: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(COLOR_GREEN)),
        Span::raw(format!(" {} • ", confirm_text)),
        Span::styled("[Esc]", Style::default().fg(COLOR_RED)),
        Span::raw(format!(" {}", cancel_text)),
    ])
}

// Helper functions for consistent styling
pub fn create_styled_span(text: &str, color: Color, bold: bool) -> Span {
    let mut style = Style::default().fg(color);
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    Span::styled(text, style)
}

pub fn create_default_cell_for_none(default_text: &str, is_stale: bool) -> Cell {
    let color = if is_stale {
        crate::ui::monitor::types::SessionStatus::dimmed_text_color()
    } else {
        COLOR_GRAY
    };
    Cell::from(default_text).style(Style::default().fg(color))
}

// Progress bar creation utility
pub fn create_progress_bar(percentage: u8) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_builder() {
        let builder = DialogBuilder::new("Test Dialog")
            .size(50, 20)
            .border_color(COLOR_RED);

        assert_eq!(builder.title, "Test Dialog");
        assert_eq!(builder.width, 50);
        assert_eq!(builder.height, 20);
        assert_eq!(builder.border_color, COLOR_RED);
    }

    #[test]
    fn test_table_builder() {
        let builder = TableBuilder::new()
            .headers(vec!["Col1".to_string(), "Col2".to_string()])
            .selected_bg(COLOR_BLUE);

        assert_eq!(builder.headers.len(), 2);
        assert_eq!(builder.selected_bg, COLOR_BLUE);
    }

    #[test]
    fn test_progress_bar_creation() {
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
    fn test_styled_span_creation() {
        let span = create_styled_span("Test", COLOR_GREEN, true);
        assert_eq!(span.content, "Test");
        assert_eq!(span.style.fg, Some(COLOR_GREEN));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
    }
}
