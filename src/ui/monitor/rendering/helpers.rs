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
    bar.push_str(&format!("{percentage}%"));

    bar
}

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
        Span::raw(format!(" {confirm_text} • ")),
        Span::styled("[Esc]", Style::default().fg(COLOR_RED)),
        Span::raw(format!(" {cancel_text}")),
    ])
}

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
