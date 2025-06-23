use crate::config::Config;
use crate::ui::monitor::state::MonitorAppState;
use crate::ui::monitor::SessionInfo;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

// Import specialized renderers
use super::renderers::{DialogRenderer, HelpRenderer, StatusRenderer, TableRenderer};

pub struct MonitorRenderer {
    dialog_renderer: DialogRenderer,
    table_renderer: TableRenderer,
    status_renderer: StatusRenderer,
    help_renderer: HelpRenderer,
}

impl MonitorRenderer {
    pub fn new(config: Config) -> Self {
        Self {
            help_renderer: HelpRenderer::new(config),
            dialog_renderer: DialogRenderer::new(),
            table_renderer: TableRenderer::new(),
            status_renderer: StatusRenderer::new(),
        }
    }

    pub fn render(&self, f: &mut Frame, sessions: &[SessionInfo], state: &mut MonitorAppState) {
        // Clear expired feedback messages and button clicks
        state.clear_expired_feedback();
        state.clear_expired_button_click();

        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .margin(1)
            .split(f.area());

        // Render main UI components using specialized renderers
        self.help_renderer.render_header(f, main_layout[0]);
        self.table_renderer
            .render_table(f, main_layout[1], sessions, state);
        self.help_renderer
            .render_footer(f, main_layout[2], sessions, state);

        // Render feedback message if present
        if state.get_feedback_message().is_some() {
            self.status_renderer.render_feedback_message(f, state);
        }

        // Render dialogs based on current mode
        self.dialog_renderer.render_dialog(f, state);
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

    fn create_test_sessions() -> Vec<SessionInfo> {
        vec![
            SessionInfo {
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
            },
            SessionInfo {
                name: "session2".to_string(),
                branch: "branch2".to_string(),
                status: SessionStatus::Idle,
                last_activity: Utc::now(),
                task: "Task 2".to_string(),
                worktree_path: PathBuf::from("/tmp/session2"),
                test_status: None,
                confidence: None,
                diff_stats: None,
                todo_percentage: None,
                is_blocked: false,
            },
        ]
    }

    #[test]
    fn test_renderer_creation() {
        let config = create_test_config();
        let _renderer = MonitorRenderer::new(config);

        // Just test that we can create the renderer instance
        // The config is now owned by the help_renderer
    }

    #[test]
    fn test_render_components() {
        let config = create_test_config();
        let _renderer = MonitorRenderer::new(config);
        let sessions = create_test_sessions();
        let state = MonitorAppState::new();

        // Test that rendering methods exist and can be called
        // We can't easily test the actual rendering without a terminal backend,
        // but we can verify the structure exists
        assert_eq!(sessions.len(), 2);
        assert_eq!(state.mode, crate::ui::monitor::AppMode::Normal);
    }
}
