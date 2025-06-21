/// Common UI patterns and reusable components for the UI monitoring system
/// This module extracts repeated patterns to reduce duplication and improve maintainability
use crate::core::status::{ConfidenceLevel, TestStatus};
use crate::ui::monitor::data::{SessionData, SessionDataStatus};
use crate::ui::monitor::presentation::PresentationTheme;
use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row},
};

/// Common styling patterns for UI components
#[allow(dead_code)]
pub struct StylePatterns;

#[allow(dead_code)]
impl StylePatterns {
    /// Create a standardized block with borders and title
    pub fn create_bordered_block<'a>(title: &'a str, border_color: Color) -> Block<'a> {
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
    }

    /// Create a standardized popup block for dialogs
    pub fn create_popup_block<'a>(title: &'a str, theme: &PresentationTheme) -> Block<'a> {
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_color))
            .style(Style::default().bg(theme.background_selected_color))
    }

    /// Create a highlighted style for important elements
    pub fn create_highlight_style(theme: &PresentationTheme) -> Style {
        Style::default()
            .fg(theme.text_selected_color)
            .bg(theme.background_selected_color)
            .add_modifier(Modifier::BOLD)
    }

    /// Create a dimmed style for inactive elements
    pub fn create_dimmed_style(theme: &PresentationTheme) -> Style {
        Style::default().fg(theme.text_dimmed_color)
    }

    /// Create a standard error style
    pub fn create_error_style(theme: &PresentationTheme) -> Style {
        Style::default()
            .fg(theme.test_failed_color)
            .add_modifier(Modifier::BOLD)
    }

    /// Create a standard success style
    pub fn create_success_style(theme: &PresentationTheme) -> Style {
        Style::default()
            .fg(theme.test_passed_color)
            .add_modifier(Modifier::BOLD)
    }
}

/// Common cell creation patterns for table components
#[allow(dead_code)]
pub struct CellPatterns;

#[allow(dead_code)]
impl CellPatterns {
    /// Create a cell with conditional styling based on state
    pub fn create_conditional_cell(
        content: impl Into<String>,
        is_selected: bool,
        is_dimmed: bool,
        theme: &PresentationTheme,
    ) -> Cell<'static> {
        let style = if is_selected {
            StylePatterns::create_highlight_style(theme)
        } else if is_dimmed {
            StylePatterns::create_dimmed_style(theme)
        } else {
            Style::default().fg(theme.text_normal_color)
        };

        Cell::from(content.into()).style(style)
    }

    /// Create a status cell with appropriate coloring
    pub fn create_status_cell(
        status: SessionDataStatus,
        is_blocked: bool,
        is_selected: bool,
        theme: &PresentationTheme,
    ) -> Cell<'static> {
        let (text, color) = if is_blocked {
            ("Blocked", theme.test_failed_color)
        } else {
            (status.as_str(), Self::status_to_color(status, theme))
        };

        let style = if is_selected {
            Style::default()
                .bg(theme.background_selected_color)
                .fg(theme.text_selected_color)
        } else {
            Style::default().fg(color)
        };

        Cell::from(text).style(style)
    }

    /// Create a test status cell with proper coloring
    pub fn create_test_status_cell(
        test_status: Option<TestStatus>,
        is_selected: bool,
        is_dimmed: bool,
        theme: &PresentationTheme,
    ) -> Cell<'static> {
        match test_status {
            Some(status) => {
                let (text, color) = Self::test_status_to_display(status, is_dimmed, theme);
                let cell_style = if is_selected {
                    StylePatterns::create_highlight_style(theme)
                } else {
                    Style::default().fg(color)
                };
                Cell::from(text).style(cell_style)
            }
            None => Self::create_empty_cell(is_selected, is_dimmed, theme),
        }
    }

    /// Create a confidence level cell with proper coloring
    pub fn create_confidence_cell(
        confidence: Option<ConfidenceLevel>,
        is_selected: bool,
        is_dimmed: bool,
        theme: &PresentationTheme,
    ) -> Cell<'static> {
        match confidence {
            Some(level) => {
                let (text, color) = Self::confidence_to_display(level, is_dimmed, theme);
                let cell_style = if is_selected {
                    StylePatterns::create_highlight_style(theme)
                } else {
                    Style::default().fg(color)
                };
                Cell::from(text).style(cell_style)
            }
            None => Self::create_empty_cell(is_selected, is_dimmed, theme),
        }
    }

    /// Create a progress cell with visual progress bar
    pub fn create_progress_cell(
        percentage: Option<u8>,
        is_selected: bool,
        is_dimmed: bool,
        theme: &PresentationTheme,
    ) -> Cell<'static> {
        match percentage {
            Some(pct) => {
                let progress_bar = ProgressPatterns::create_progress_bar(pct, 8);
                let color = if is_selected {
                    theme.text_selected_color
                } else if is_dimmed {
                    theme.text_dimmed_color
                } else {
                    Self::progress_to_color(pct, theme)
                };
                Cell::from(progress_bar).style(Style::default().fg(color))
            }
            None => {
                let style = if is_selected {
                    StylePatterns::create_highlight_style(theme)
                } else if is_dimmed {
                    StylePatterns::create_dimmed_style(theme)
                } else {
                    Style::default().fg(theme.border_color)
                };
                Cell::from("░░░░░░░░ ─").style(style)
            }
        }
    }

    /// Create an empty cell with appropriate styling for missing data
    pub fn create_empty_cell(
        is_selected: bool,
        is_dimmed: bool,
        theme: &PresentationTheme,
    ) -> Cell<'static> {
        let style = if is_selected {
            StylePatterns::create_highlight_style(theme)
        } else if is_dimmed {
            StylePatterns::create_dimmed_style(theme)
        } else {
            Style::default().fg(theme.border_color)
        };
        Cell::from("-").style(style)
    }

    // Helper methods for color and text mapping
    fn status_to_color(status: SessionDataStatus, theme: &PresentationTheme) -> Color {
        match status {
            SessionDataStatus::Active => theme.active_color,
            SessionDataStatus::Idle => theme.idle_color,
            SessionDataStatus::Review => theme.review_color,
            SessionDataStatus::Ready => theme.ready_color,
            SessionDataStatus::Stale => theme.stale_color,
        }
    }

    fn test_status_to_display(
        status: TestStatus,
        is_dimmed: bool,
        theme: &PresentationTheme,
    ) -> (&'static str, Color) {
        let color = if is_dimmed {
            theme.text_dimmed_color
        } else {
            match status {
                TestStatus::Passed => theme.test_passed_color,
                TestStatus::Failed => theme.test_failed_color,
                TestStatus::Unknown => theme.test_unknown_color,
            }
        };

        let text = match status {
            TestStatus::Passed => "Passed",
            TestStatus::Failed => "Failed",
            TestStatus::Unknown => "Unknown",
        };

        (text, color)
    }

    fn confidence_to_display(
        level: ConfidenceLevel,
        is_dimmed: bool,
        theme: &PresentationTheme,
    ) -> (&'static str, Color) {
        let color = if is_dimmed {
            theme.text_dimmed_color
        } else {
            match level {
                ConfidenceLevel::High => theme.confidence_high_color,
                ConfidenceLevel::Medium => theme.confidence_medium_color,
                ConfidenceLevel::Low => theme.confidence_low_color,
            }
        };

        let text = match level {
            ConfidenceLevel::High => "High",
            ConfidenceLevel::Medium => "Medium",
            ConfidenceLevel::Low => "Low",
        };

        (text, color)
    }

    fn progress_to_color(percentage: u8, theme: &PresentationTheme) -> Color {
        if percentage == 100 {
            theme.progress_complete_color
        } else if percentage >= 50 {
            theme.progress_high_color
        } else {
            theme.progress_medium_color
        }
    }
}

/// Progress bar creation patterns
#[allow(dead_code)]
pub struct ProgressPatterns;

#[allow(dead_code)]
impl ProgressPatterns {
    /// Create a visual progress bar with given percentage and width
    pub fn create_progress_bar(percentage: u8, width: usize) -> String {
        let filled = (percentage as f32 / 100.0 * width as f32).round() as usize;
        let filled = filled.min(width);

        let mut bar = String::with_capacity(width + 5);

        for _ in 0..filled {
            bar.push('█');
        }

        for _ in filled..width {
            bar.push('░');
        }

        bar.push(' ');
        bar.push_str(&format!("{}%", percentage));

        bar
    }

    /// Create a simple percentage display without visual bar
    pub fn create_percentage_text(percentage: u8) -> String {
        format!("{}%", percentage)
    }

    /// Create a completion indicator (done/total format)
    pub fn create_completion_indicator(completed: usize, total: usize) -> String {
        format!("{}/{}", completed, total)
    }
}

/// Row creation patterns for tables
#[allow(dead_code)]
pub struct RowPatterns;

#[allow(dead_code)]
impl RowPatterns {
    /// Create a standardized session row using the extracted patterns
    pub fn create_session_row(
        session_data: &SessionData,
        index: usize,
        selected_index: usize,
        theme: &PresentationTheme,
    ) -> Row<'static> {
        let is_selected = index == selected_index;
        let is_dimmed = session_data.status.is_inactive();

        Row::new(vec![
            // Session name - always bold
            CellPatterns::create_conditional_cell(
                &session_data.name,
                is_selected,
                is_dimmed,
                theme,
            )
            .style(Style::default().add_modifier(Modifier::BOLD)),
            // Status with appropriate coloring
            CellPatterns::create_status_cell(
                session_data.status,
                session_data.is_blocked,
                is_selected,
                theme,
            ),
            // Activity time
            CellPatterns::create_conditional_cell(
                crate::ui::monitor::format_activity(&session_data.last_activity),
                is_selected,
                is_dimmed,
                theme,
            ),
            // Task (truncated)
            CellPatterns::create_conditional_cell(
                crate::ui::monitor::truncate_task(&session_data.task, 40),
                is_selected,
                is_dimmed,
                theme,
            ),
            // Test status
            CellPatterns::create_test_status_cell(
                session_data.test_status.clone(),
                is_selected,
                is_dimmed,
                theme,
            ),
            // Progress
            CellPatterns::create_progress_cell(
                session_data.todo_percentage,
                is_selected,
                is_dimmed,
                theme,
            ),
            // Confidence
            CellPatterns::create_confidence_cell(
                session_data.confidence.clone(),
                is_selected,
                is_dimmed,
                theme,
            ),
        ])
        .height(1)
    }

    /// Create a standardized header row for session tables
    pub fn create_session_header_row(theme: &PresentationTheme) -> Row<'static> {
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
                .fg(theme.text_normal_color)
                .add_modifier(Modifier::BOLD),
        )
    }
}

/// Validation patterns for common input validation
#[allow(dead_code)]
pub struct ValidationPatterns;

#[allow(dead_code)]
impl ValidationPatterns {
    /// Validate session name format
    pub fn validate_session_name(name: &str) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Session name cannot be empty".to_string());
        }
        if name.len() > 100 {
            return Err("Session name too long (max 100 characters)".to_string());
        }
        if name.contains(|c: char| c.is_control() || c == '\n' || c == '\t') {
            return Err("Session name contains invalid characters".to_string());
        }
        Ok(())
    }

    /// Validate prompt text
    pub fn validate_prompt(prompt: &str) -> Result<(), String> {
        if prompt.trim().is_empty() {
            return Err("Prompt cannot be empty".to_string());
        }
        if prompt.len() > 10000 {
            return Err("Prompt too long (max 10000 characters)".to_string());
        }
        Ok(())
    }

    /// Validate percentage value
    pub fn validate_percentage(value: u8) -> Result<(), String> {
        if value > 100 {
            return Err("Percentage must be between 0 and 100".to_string());
        }
        Ok(())
    }

    /// Validate selection index
    pub fn validate_selection_index(index: usize, max_index: usize) -> Result<(), String> {
        if index >= max_index {
            return Err(format!(
                "Selection index {} out of range (max: {})",
                index, max_index
            ));
        }
        Ok(())
    }
}

/// Builder pattern for creating complex UI components
#[allow(dead_code)]
pub struct ComponentBuilder {
    theme: PresentationTheme,
}

#[allow(dead_code)]
impl ComponentBuilder {
    /// Create a new component builder with theme
    pub fn new(theme: PresentationTheme) -> Self {
        Self { theme }
    }

    /// Build a complete session row with all styling applied
    pub fn build_session_row(
        &self,
        session_data: &SessionData,
        is_selected: bool,
        is_current: bool,
    ) -> Row<'static> {
        let is_dimmed = session_data.status.is_inactive();

        // Apply additional styling for current session
        let base_style = if is_current {
            Style::default()
                .fg(self.theme.active_color)
                .add_modifier(Modifier::UNDERLINED)
        } else if is_selected {
            StylePatterns::create_highlight_style(&self.theme)
        } else if is_dimmed {
            StylePatterns::create_dimmed_style(&self.theme)
        } else {
            Style::default().fg(self.theme.text_normal_color)
        };

        Row::new(vec![
            Cell::from(session_data.name.clone()).style(base_style.add_modifier(Modifier::BOLD)),
            CellPatterns::create_status_cell(
                session_data.status,
                session_data.is_blocked,
                is_selected,
                &self.theme,
            ),
            Cell::from(crate::ui::monitor::format_activity(
                &session_data.last_activity,
            ))
            .style(base_style),
            Cell::from(crate::ui::monitor::truncate_task(&session_data.task, 40)).style(base_style),
            CellPatterns::create_test_status_cell(
                session_data.test_status.clone(),
                is_selected,
                is_dimmed,
                &self.theme,
            ),
            CellPatterns::create_progress_cell(
                session_data.todo_percentage,
                is_selected,
                is_dimmed,
                &self.theme,
            ),
            CellPatterns::create_confidence_cell(
                session_data.confidence.clone(),
                is_selected,
                is_dimmed,
                &self.theme,
            ),
        ])
        .height(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;

    fn create_test_theme() -> PresentationTheme {
        PresentationTheme::default()
    }

    fn create_test_session_data() -> SessionData {
        SessionData::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            SessionDataStatus::Active,
            Utc::now(),
            "Test task".to_string(),
            PathBuf::from("/test/path"),
        )
    }

    #[test]
    fn test_style_patterns() {
        let theme = create_test_theme();

        // Test block creation
        let _block = StylePatterns::create_bordered_block("Test", theme.border_color);
        // Can't directly test block properties due to private fields

        let _popup = StylePatterns::create_popup_block("Popup", &theme);
        let _highlight = StylePatterns::create_highlight_style(&theme);
        let _dimmed = StylePatterns::create_dimmed_style(&theme);
        let _error = StylePatterns::create_error_style(&theme);
        let _success = StylePatterns::create_success_style(&theme);

        // If we reach here without panicking, the patterns work
    }

    #[test]
    fn test_cell_patterns() {
        let theme = create_test_theme();

        // Test conditional cell
        let _cell = CellPatterns::create_conditional_cell("content", false, false, &theme);
        let _selected_cell = CellPatterns::create_conditional_cell("content", true, false, &theme);
        let _dimmed_cell = CellPatterns::create_conditional_cell("content", false, true, &theme);

        // Test status cell
        let _status_cell =
            CellPatterns::create_status_cell(SessionDataStatus::Active, false, false, &theme);

        let _blocked_cell =
            CellPatterns::create_status_cell(SessionDataStatus::Active, true, false, &theme);

        // Test test status cell
        let _test_cell =
            CellPatterns::create_test_status_cell(Some(TestStatus::Passed), false, false, &theme);

        let _no_test_cell = CellPatterns::create_test_status_cell(None, false, false, &theme);

        // Test confidence cell
        let _confidence_cell =
            CellPatterns::create_confidence_cell(Some(ConfidenceLevel::High), false, false, &theme);

        // Test progress cell
        let _progress_cell = CellPatterns::create_progress_cell(Some(75), false, false, &theme);

        let _empty_progress = CellPatterns::create_progress_cell(None, false, false, &theme);

        // Test empty cell
        let _empty_cell = CellPatterns::create_empty_cell(false, false, &theme);
    }

    #[test]
    fn test_progress_patterns() {
        let bar = ProgressPatterns::create_progress_bar(50, 10);
        assert!(bar.contains("50%"));
        assert!(bar.contains("█"));
        assert!(bar.contains("░"));

        let text = ProgressPatterns::create_percentage_text(75);
        assert_eq!(text, "75%");

        let completion = ProgressPatterns::create_completion_indicator(3, 5);
        assert_eq!(completion, "3/5");
    }

    #[test]
    fn test_row_patterns() {
        let theme = create_test_theme();
        let session_data = create_test_session_data();

        let _row = RowPatterns::create_session_row(&session_data, 0, 1, &theme);
        let _selected_row = RowPatterns::create_session_row(&session_data, 1, 1, &theme);

        let _header = RowPatterns::create_session_header_row(&theme);
    }

    #[test]
    fn test_validation_patterns() {
        // Test session name validation
        assert!(ValidationPatterns::validate_session_name("valid-name").is_ok());
        assert!(ValidationPatterns::validate_session_name("").is_err());
        assert!(ValidationPatterns::validate_session_name("a".repeat(101).as_str()).is_err());
        assert!(ValidationPatterns::validate_session_name("name\nwith\nnewlines").is_err());

        // Test prompt validation
        assert!(ValidationPatterns::validate_prompt("valid prompt").is_ok());
        assert!(ValidationPatterns::validate_prompt("").is_err());
        assert!(ValidationPatterns::validate_prompt("a".repeat(10001).as_str()).is_err());

        // Test percentage validation
        assert!(ValidationPatterns::validate_percentage(50).is_ok());
        assert!(ValidationPatterns::validate_percentage(100).is_ok());
        assert!(ValidationPatterns::validate_percentage(101).is_err());

        // Test index validation
        assert!(ValidationPatterns::validate_selection_index(0, 5).is_ok());
        assert!(ValidationPatterns::validate_selection_index(4, 5).is_ok());
        assert!(ValidationPatterns::validate_selection_index(5, 5).is_err());
    }

    #[test]
    fn test_component_builder() {
        let theme = create_test_theme();
        let session_data = create_test_session_data();
        let builder = ComponentBuilder::new(theme);

        let _normal_row = builder.build_session_row(&session_data, false, false);
        let _selected_row = builder.build_session_row(&session_data, true, false);
        let _current_row = builder.build_session_row(&session_data, false, true);
    }
}
