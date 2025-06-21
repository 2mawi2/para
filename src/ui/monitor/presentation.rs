#![allow(dead_code)] // Allow dead code during Phase 2 development

/// Presentation layer for UI monitoring system  
/// This module handles all visual styling, colors, and presentation logic
use crate::core::status::{ConfidenceLevel, TestStatus};
use crate::ui::monitor::data::{SessionData, SessionDataStatus};
use ratatui::style::{Color, Modifier, Style};

/// Presentation configuration for UI themes and styling
#[derive(Debug, Clone)]
pub struct PresentationTheme {
    // Status colors
    pub active_color: Color,
    pub idle_color: Color,
    pub review_color: Color,
    pub ready_color: Color,
    pub stale_color: Color,

    // Test status colors
    pub test_passed_color: Color,
    pub test_failed_color: Color,
    pub test_unknown_color: Color,

    // Confidence colors
    pub confidence_high_color: Color,
    pub confidence_medium_color: Color,
    pub confidence_low_color: Color,

    // Progress colors
    pub progress_complete_color: Color,
    pub progress_high_color: Color,
    pub progress_medium_color: Color,
    pub progress_low_color: Color,

    // UI colors
    pub text_normal_color: Color,
    pub text_dimmed_color: Color,
    pub text_selected_color: Color,
    pub background_selected_color: Color,
    pub border_color: Color,
}

impl Default for PresentationTheme {
    fn default() -> Self {
        Self {
            // Status colors
            active_color: Color::Rgb(34, 197, 94),  // Green
            idle_color: Color::Rgb(245, 158, 11),   // Amber
            review_color: Color::Rgb(147, 51, 234), // Purple
            ready_color: Color::Rgb(99, 102, 241),  // Indigo
            stale_color: Color::Rgb(107, 114, 128), // Gray

            // Test status colors
            test_passed_color: Color::Rgb(34, 197, 94), // Green
            test_failed_color: Color::Rgb(239, 68, 68), // Red
            test_unknown_color: Color::Rgb(156, 163, 175), // Light gray

            // Confidence colors
            confidence_high_color: Color::Rgb(34, 197, 94), // Green
            confidence_medium_color: Color::Rgb(245, 158, 11), // Orange
            confidence_low_color: Color::Rgb(239, 68, 68),  // Red

            // Progress colors
            progress_complete_color: Color::Rgb(34, 197, 94), // Green
            progress_high_color: Color::Rgb(99, 102, 241),    // Blue
            progress_medium_color: Color::Rgb(245, 158, 11),  // Orange
            progress_low_color: Color::Rgb(245, 158, 11),     // Orange

            // UI colors
            text_normal_color: Color::Rgb(229, 231, 235), // Light gray
            text_dimmed_color: Color::Rgb(75, 85, 99),    // Dark gray
            text_selected_color: Color::Rgb(255, 255, 255), // White
            background_selected_color: Color::Rgb(30, 41, 59), // Dark slate
            border_color: Color::Rgb(75, 85, 99),         // Gray
        }
    }
}

/// Session presentation model that handles styling and visual representation
#[derive(Debug, Clone)]
pub struct SessionViewModel {
    pub data: SessionData,
    pub theme: PresentationTheme,
    pub is_selected: bool,
    pub is_current_session: bool,
}

impl SessionViewModel {
    /// Create a new SessionViewModel from data
    pub fn new(data: SessionData) -> Self {
        Self {
            data,
            theme: PresentationTheme::default(),
            is_selected: false,
            is_current_session: false,
        }
    }

    /// Create with custom theme
    pub fn with_theme(data: SessionData, theme: PresentationTheme) -> Self {
        Self {
            data,
            theme,
            is_selected: false,
            is_current_session: false,
        }
    }

    /// Set selection state
    pub fn with_selection(mut self, is_selected: bool) -> Self {
        self.is_selected = is_selected;
        self
    }

    /// Set current session state
    pub fn with_current_session(mut self, is_current: bool) -> Self {
        self.is_current_session = is_current;
        self
    }

    /// Get the color for the session status
    pub fn status_color(&self) -> Color {
        match self.data.status {
            SessionDataStatus::Active => self.theme.active_color,
            SessionDataStatus::Idle => self.theme.idle_color,
            SessionDataStatus::Review => self.theme.review_color,
            SessionDataStatus::Ready => self.theme.ready_color,
            SessionDataStatus::Stale => self.theme.stale_color,
        }
    }

    /// Get the color for test status
    pub fn test_status_color(&self) -> Option<Color> {
        match &self.data.test_status {
            Some(TestStatus::Passed) => Some(if self.should_dim() {
                self.theme.text_dimmed_color
            } else {
                self.theme.test_passed_color
            }),
            Some(TestStatus::Failed) => Some(if self.should_dim() {
                self.theme.text_dimmed_color
            } else {
                self.theme.test_failed_color
            }),
            Some(TestStatus::Unknown) => Some(if self.should_dim() {
                self.theme.text_dimmed_color
            } else {
                self.theme.test_unknown_color
            }),
            None => None,
        }
    }

    /// Get the color for confidence level
    pub fn confidence_color(&self) -> Option<Color> {
        match &self.data.confidence {
            Some(ConfidenceLevel::High) => Some(if self.should_dim() {
                self.theme.text_dimmed_color
            } else {
                self.theme.confidence_high_color
            }),
            Some(ConfidenceLevel::Medium) => Some(if self.should_dim() {
                self.theme.text_dimmed_color
            } else {
                self.theme.confidence_medium_color
            }),
            Some(ConfidenceLevel::Low) => Some(if self.should_dim() {
                self.theme.text_dimmed_color
            } else {
                self.theme.confidence_low_color
            }),
            None => None,
        }
    }

    /// Get the color for progress percentage
    pub fn progress_color(&self) -> Color {
        if self.should_dim() {
            self.theme.text_dimmed_color
        } else {
            let percentage = self.data.completion_percentage();
            if percentage == 100 {
                self.theme.progress_complete_color
            } else if percentage >= 50 {
                self.theme.progress_high_color
            } else {
                self.theme.progress_medium_color
            }
        }
    }

    /// Get the base text style for this session
    pub fn base_text_style(&self) -> Style {
        if self.is_selected {
            Style::default()
                .bg(self.theme.background_selected_color)
                .fg(self.theme.text_selected_color)
        } else if self.should_dim() {
            Style::default().fg(self.theme.text_dimmed_color)
        } else {
            Style::default().fg(self.theme.text_normal_color)
        }
    }

    /// Get the style for the session name (usually bold)
    pub fn name_style(&self) -> Style {
        self.base_text_style().add_modifier(Modifier::BOLD)
    }

    /// Get the style for status display
    pub fn status_style(&self) -> Style {
        let color = if self.data.is_blocked {
            self.theme.test_failed_color // Use red for blocked sessions
        } else {
            self.status_color()
        };
        Style::default().fg(color)
    }

    /// Returns true if this session should be dimmed/transparent
    pub fn should_dim(&self) -> bool {
        self.data.status.is_inactive()
    }

    /// Get display text for status (handles blocked state)
    pub fn status_display_text(&self) -> &str {
        if self.data.is_blocked {
            "Blocked"
        } else {
            self.data.status.as_str()
        }
    }

    /// Get display text for test status
    pub fn test_status_display_text(&self) -> Option<&'static str> {
        match &self.data.test_status {
            Some(TestStatus::Passed) => Some("Passed"),
            Some(TestStatus::Failed) => Some("Failed"),
            Some(TestStatus::Unknown) => Some("Unknown"),
            None => None,
        }
    }

    /// Get display text for confidence level
    pub fn confidence_display_text(&self) -> Option<&'static str> {
        match &self.data.confidence {
            Some(ConfidenceLevel::High) => Some("High"),
            Some(ConfidenceLevel::Medium) => Some("Medium"),
            Some(ConfidenceLevel::Low) => Some("Low"),
            None => None,
        }
    }

    /// Create a progress bar string with the given width
    pub fn create_progress_bar(&self, width: usize) -> String {
        let percentage = self.data.completion_percentage();
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
}

/// Utility functions for presentation layer
pub struct PresentationUtils;

impl PresentationUtils {
    /// Create a default cell style for None/empty values
    pub fn default_none_cell_style(theme: &PresentationTheme, is_stale: bool) -> Style {
        let color = if is_stale {
            theme.text_dimmed_color
        } else {
            theme.border_color
        };
        Style::default().fg(color)
    }

    /// Format an activity timestamp for display
    pub fn format_activity_time(time: &chrono::DateTime<chrono::Utc>) -> String {
        crate::ui::monitor::format_activity(time)
    }

    /// Truncate task text to fit in display
    pub fn truncate_task_text(task: &str, max_length: usize) -> String {
        crate::ui::monitor::truncate_task(task, max_length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::monitor::data::SessionData;
    use chrono::Utc;
    use std::path::PathBuf;

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
    fn test_presentation_theme_default() {
        let theme = PresentationTheme::default();
        assert_eq!(theme.active_color, Color::Rgb(34, 197, 94));
        assert_eq!(theme.test_passed_color, Color::Rgb(34, 197, 94));
        assert_eq!(theme.text_normal_color, Color::Rgb(229, 231, 235));
    }

    #[test]
    fn test_session_view_model_creation() {
        let data = create_test_session_data();
        let view_model = SessionViewModel::new(data.clone());

        assert_eq!(view_model.data.name, "test-session");
        assert!(!view_model.is_selected);
        assert!(!view_model.is_current_session);
    }

    #[test]
    fn test_session_view_model_fluent_api() {
        let data = create_test_session_data();
        let view_model = SessionViewModel::new(data)
            .with_selection(true)
            .with_current_session(true);

        assert!(view_model.is_selected);
        assert!(view_model.is_current_session);
    }

    #[test]
    fn test_status_color_mapping() {
        let mut data = create_test_session_data();
        let theme = PresentationTheme::default();

        data.status = SessionDataStatus::Active;
        let view_model = SessionViewModel::with_theme(data.clone(), theme.clone());
        assert_eq!(view_model.status_color(), theme.active_color);

        data.status = SessionDataStatus::Review;
        let view_model = SessionViewModel::with_theme(data, theme.clone());
        assert_eq!(view_model.status_color(), theme.review_color);
    }

    #[test]
    fn test_test_status_color_with_dimming() {
        let mut data = create_test_session_data();
        let theme = PresentationTheme::default();

        // Test normal (non-dimmed) color
        data.test_status = Some(TestStatus::Passed);
        let view_model = SessionViewModel::with_theme(data.clone(), theme.clone());
        assert_eq!(
            view_model.test_status_color(),
            Some(theme.test_passed_color)
        );

        // Test dimmed color for stale sessions
        data.status = SessionDataStatus::Stale;
        let view_model = SessionViewModel::with_theme(data, theme.clone());
        assert_eq!(
            view_model.test_status_color(),
            Some(theme.text_dimmed_color)
        );
    }

    #[test]
    fn test_confidence_color_mapping() {
        let mut data = create_test_session_data();
        let theme = PresentationTheme::default();

        data.confidence = Some(ConfidenceLevel::High);
        let view_model = SessionViewModel::with_theme(data.clone(), theme.clone());
        assert_eq!(
            view_model.confidence_color(),
            Some(theme.confidence_high_color)
        );

        data.confidence = Some(ConfidenceLevel::Low);
        let view_model = SessionViewModel::with_theme(data, theme.clone());
        assert_eq!(
            view_model.confidence_color(),
            Some(theme.confidence_low_color)
        );
    }

    #[test]
    fn test_progress_color_logic() {
        let mut data = create_test_session_data();
        let theme = PresentationTheme::default();

        // Test complete progress
        data.todo_percentage = Some(100);
        let view_model = SessionViewModel::with_theme(data.clone(), theme.clone());
        assert_eq!(view_model.progress_color(), theme.progress_complete_color);

        // Test high progress
        data.todo_percentage = Some(75);
        let view_model = SessionViewModel::with_theme(data.clone(), theme.clone());
        assert_eq!(view_model.progress_color(), theme.progress_high_color);

        // Test low progress
        data.todo_percentage = Some(25);
        let view_model = SessionViewModel::with_theme(data, theme.clone());
        assert_eq!(view_model.progress_color(), theme.progress_medium_color);
    }

    #[test]
    fn test_base_text_style_selection() {
        let data = create_test_session_data();
        let theme = PresentationTheme::default();

        // Test normal style
        let view_model = SessionViewModel::with_theme(data.clone(), theme.clone());
        let style = view_model.base_text_style();
        assert_eq!(style.fg, Some(theme.text_normal_color));

        // Test selected style
        let view_model = SessionViewModel::with_theme(data, theme.clone()).with_selection(true);
        let style = view_model.base_text_style();
        assert_eq!(style.bg, Some(theme.background_selected_color));
        assert_eq!(style.fg, Some(theme.text_selected_color));
    }

    #[test]
    fn test_dimming_logic() {
        let mut data = create_test_session_data();
        let theme = PresentationTheme::default();

        // Active session should not dim
        data.status = SessionDataStatus::Active;
        let view_model = SessionViewModel::with_theme(data.clone(), theme.clone());
        assert!(!view_model.should_dim());

        // Stale session should dim
        data.status = SessionDataStatus::Stale;
        let view_model = SessionViewModel::with_theme(data, theme);
        assert!(view_model.should_dim());
    }

    #[test]
    fn test_blocked_session_display() {
        let mut data = create_test_session_data();
        data.is_blocked = true;
        let view_model = SessionViewModel::new(data);

        assert_eq!(view_model.status_display_text(), "Blocked");
        assert_eq!(
            view_model.status_style().fg,
            Some(view_model.theme.test_failed_color)
        );
    }

    #[test]
    fn test_display_text_methods() {
        let mut data = create_test_session_data();
        data.test_status = Some(TestStatus::Failed);
        data.confidence = Some(ConfidenceLevel::Medium);

        let view_model = SessionViewModel::new(data);

        assert_eq!(view_model.test_status_display_text(), Some("Failed"));
        assert_eq!(view_model.confidence_display_text(), Some("Medium"));
    }

    #[test]
    fn test_progress_bar_creation() {
        let mut data = create_test_session_data();
        data.todo_percentage = Some(50);

        let view_model = SessionViewModel::new(data);
        let progress_bar = view_model.create_progress_bar(8);

        assert!(progress_bar.contains("████░░░░"));
        assert!(progress_bar.contains("50%"));
    }

    #[test]
    fn test_presentation_utils() {
        let theme = PresentationTheme::default();

        // Test default cell style
        let style = PresentationUtils::default_none_cell_style(&theme, false);
        assert_eq!(style.fg, Some(theme.border_color));

        let style = PresentationUtils::default_none_cell_style(&theme, true);
        assert_eq!(style.fg, Some(theme.text_dimmed_color));
    }
}
