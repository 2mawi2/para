use crate::config::Config;
use crate::core::session::SessionManager;
use crate::ui::monitor::state::MonitorAppState;
use crate::ui::monitor::SessionInfo;
use ratatui::{
    layout::Alignment,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::path::PathBuf;

use super::components::{
    create_styled_span, COLOR_BLUE, COLOR_BORDER, COLOR_LIGHT_GRAY, COLOR_WHITE,
};

pub struct HelpRenderer {
    config: Config,
}

impl HelpRenderer {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Render the application header with title and auto-refresh info
    pub fn render_header(&self, f: &mut Frame, area: Rect) {
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

    /// Render the application footer with session info and keyboard shortcuts
    pub fn render_footer(
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

    fn _create_test_sessions() -> Vec<SessionInfo> {
        vec![SessionInfo {
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
        }]
    }

    #[test]
    fn test_help_renderer_creation() {
        let config = create_test_config();
        let renderer = HelpRenderer::new(config);

        // Just test that we can create the renderer instance
        assert_eq!(renderer.config.git.branch_prefix, "para");
    }

    // Note: Actual rendering tests would require a terminal backend
    // These tests verify the structure exists and can be instantiated
}
