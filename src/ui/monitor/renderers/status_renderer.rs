use crate::core::status::{ConfidenceLevel, DiffStats, TestStatus};
use crate::ui::monitor::state::MonitorAppState;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Paragraph},
    Frame,
};

use super::components::{
    create_default_cell_for_none, create_progress_bar, COLOR_BLACK, COLOR_BLUE, COLOR_GREEN,
    COLOR_LIGHT_GRAY, COLOR_ORANGE, COLOR_RED,
};

pub struct StatusRenderer;

impl StatusRenderer {
    pub fn new() -> Self {
        Self
    }

    /// Create a progress cell with percentage bar
    pub fn create_progress_cell(&self, todo_percentage: Option<u8>, is_stale: bool) -> Cell {
        match todo_percentage {
            Some(pct) => {
                let progress_bar = create_progress_bar(pct);
                let color = self.get_progress_color(pct, is_stale);
                Cell::from(progress_bar).style(Style::default().fg(color))
            }
            None => create_default_cell_for_none("░░░░░░░░ ─", is_stale),
        }
    }

    /// Get appropriate color for progress based on percentage and staleness
    pub fn get_progress_color(&self, percentage: u8, is_stale: bool) -> Color {
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

    /// Create a test status cell with appropriate styling
    pub fn create_test_cell(&self, test_status: &Option<TestStatus>, is_stale: bool) -> Cell {
        match test_status {
            Some(status) => {
                let (text, color) = self.get_test_status_display(status, is_stale);
                Cell::from(text).style(Style::default().fg(color))
            }
            None => create_default_cell_for_none("-", is_stale),
        }
    }

    /// Get display text and color for test status
    pub fn get_test_status_display(
        &self,
        status: &TestStatus,
        is_stale: bool,
    ) -> (&'static str, Color) {
        let dimmed_color = crate::ui::monitor::types::SessionStatus::dimmed_text_color();

        match status {
            TestStatus::Passed => ("Passed", if is_stale { dimmed_color } else { COLOR_GREEN }),
            TestStatus::Failed => ("Failed", if is_stale { dimmed_color } else { COLOR_RED }),
            TestStatus::Unknown => (
                "Unknown",
                if is_stale {
                    dimmed_color
                } else {
                    COLOR_LIGHT_GRAY
                },
            ),
        }
    }

    /// Create a confidence level cell with appropriate styling
    pub fn create_confidence_cell(
        &self,
        confidence: &Option<ConfidenceLevel>,
        is_stale: bool,
    ) -> Cell {
        match confidence {
            Some(level) => {
                let (text, color) = self.get_confidence_display(level, is_stale);
                Cell::from(text).style(Style::default().fg(color))
            }
            None => create_default_cell_for_none("-", is_stale),
        }
    }

    /// Get display text and color for confidence level
    pub fn get_confidence_display(
        &self,
        level: &ConfidenceLevel,
        is_stale: bool,
    ) -> (&'static str, Color) {
        let dimmed_color = crate::ui::monitor::types::SessionStatus::dimmed_text_color();

        match level {
            ConfidenceLevel::High => ("High", if is_stale { dimmed_color } else { COLOR_GREEN }),
            ConfidenceLevel::Medium => {
                ("Medium", if is_stale { dimmed_color } else { COLOR_ORANGE })
            }
            ConfidenceLevel::Low => ("Low", if is_stale { dimmed_color } else { COLOR_RED }),
        }
    }

    /// Create a diff stats cell showing additions and deletions
    pub fn create_diff_stats_cell(&self, diff_stats: &Option<DiffStats>, is_stale: bool) -> Cell {
        match diff_stats {
            Some(stats) => {
                if is_stale {
                    let text = format!("+{} -{}", stats.additions, stats.deletions);
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

    /// Render feedback message as a toast notification
    pub fn render_feedback_message(&self, f: &mut Frame, state: &MonitorAppState) {
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

impl Default for StatusRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_renderer() -> StatusRenderer {
        StatusRenderer::new()
    }

    #[test]
    fn test_status_renderer_creation() {
        let renderer = create_test_renderer();
        assert_eq!(std::mem::size_of_val(&renderer), 0); // Zero-sized struct
    }

    #[test]
    fn test_get_progress_color() {
        let renderer = create_test_renderer();

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
        let renderer = create_test_renderer();

        // Test passed status
        let (text, color) = renderer.get_test_status_display(&TestStatus::Passed, false);
        assert_eq!(text, "Passed");
        assert_eq!(color, COLOR_GREEN);

        // Test failed status
        let (text, color) = renderer.get_test_status_display(&TestStatus::Failed, false);
        assert_eq!(text, "Failed");
        assert_eq!(color, COLOR_RED);

        // Test stale status override
        let (text, color) = renderer.get_test_status_display(&TestStatus::Passed, true);
        assert_eq!(text, "Passed");
        assert_eq!(
            color,
            crate::ui::monitor::types::SessionStatus::dimmed_text_color()
        );
    }

    #[test]
    fn test_get_confidence_display() {
        let renderer = create_test_renderer();

        // Test high confidence
        let (text, color) = renderer.get_confidence_display(&ConfidenceLevel::High, false);
        assert_eq!(text, "High");
        assert_eq!(color, COLOR_GREEN);

        // Test medium confidence
        let (text, color) = renderer.get_confidence_display(&ConfidenceLevel::Medium, false);
        assert_eq!(text, "Medium");
        assert_eq!(color, COLOR_ORANGE);

        // Test low confidence
        let (text, color) = renderer.get_confidence_display(&ConfidenceLevel::Low, false);
        assert_eq!(text, "Low");
        assert_eq!(color, COLOR_RED);

        // Test stale override
        let (text, color) = renderer.get_confidence_display(&ConfidenceLevel::High, true);
        assert_eq!(text, "High");
        assert_eq!(
            color,
            crate::ui::monitor::types::SessionStatus::dimmed_text_color()
        );
    }
}
