use crate::cli::parser::WatchArgs;
use crate::utils::Result;
use anyhow::Result as AnyhowResult;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};
use std::io;

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub task_name: String,
    pub agent_name: String,
    pub description: String,
    pub state: SessionState,
    pub ide_open: bool,
    pub ai_review_minutes: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionState {
    Working,
    AIReview,
    HumanReview,
}

impl SessionState {
    fn color(&self) -> Color {
        match self {
            SessionState::Working => Color::Rgb(99, 102, 241), // Modern indigo
            SessionState::AIReview => Color::Rgb(245, 158, 11), // Warm amber
            SessionState::HumanReview => Color::Rgb(236, 72, 153), // Pink
        }
    }

    #[allow(dead_code)]
    fn bg_color(&self) -> Color {
        match self {
            SessionState::Working => Color::Rgb(30, 30, 80), // Dark indigo bg
            SessionState::AIReview => Color::Rgb(80, 60, 30), // Dark amber bg
            SessionState::HumanReview => Color::Rgb(80, 30, 60), // Dark pink bg
        }
    }

    fn name(&self) -> &str {
        match self {
            SessionState::Working => "WORKING",
            SessionState::AIReview => "AI REVIEW",
            SessionState::HumanReview => "HUMAN REVIEW",
        }
    }
}

pub struct App {
    sessions: Vec<SessionInfo>,
    selected_index: usize,
    should_quit: bool,
    table_state: TableState,
    current_section: usize, // 0=working, 1=ai_review, 2=human_review
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            sessions: create_mock_sessions(),
            selected_index: 0,
            should_quit: false,
            table_state,
            current_section: 0,
        }
    }

    pub fn with_real_data(config: &crate::config::Config) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            sessions: load_sessions_with_fallback(config),
            selected_index: 0,
            should_quit: false,
            table_state,
            current_section: 0,
        }
    }

    pub fn run(&mut self) -> AnyhowResult<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal);

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> AnyhowResult<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                self.handle_key(key);
                if self.should_quit {
                    break;
                }
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('r') => {
                // Refresh functionality - could reload session data
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let num = c.to_digit(10).unwrap() as usize;
                if num > 0 && num <= self.sessions.len() {
                    let _session = &self.sessions[num - 1];
                    // In real implementation, this would open the IDE
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.previous_item();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.next_item();
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.activate_selected();
            }
            KeyCode::Tab => {
                self.next_section();
            }
            KeyCode::BackTab => {
                self.previous_section();
            }
            _ => {}
        }
    }

    fn previous_item(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.table_state.select(Some(self.selected_index));
        }
    }

    fn next_item(&mut self) {
        if self.selected_index < self.sessions.len().saturating_sub(1) {
            self.selected_index += 1;
            self.table_state.select(Some(self.selected_index));
        }
    }

    fn activate_selected(&mut self) {
        if let Some(_session) = self.sessions.get(self.selected_index) {
            // In real implementation, this would open the IDE or show details
        }
    }

    fn next_section(&mut self) {
        self.current_section = (self.current_section + 1) % 3;
        self.jump_to_section();
    }

    fn previous_section(&mut self) {
        self.current_section = if self.current_section == 0 {
            2
        } else {
            self.current_section - 1
        };
        self.jump_to_section();
    }

    fn jump_to_section(&mut self) {
        let target_state = match self.current_section {
            0 => SessionState::Working,
            1 => SessionState::AIReview,
            _ => SessionState::HumanReview,
        };

        if let Some(index) = self.sessions.iter().position(|s| s.state == target_state) {
            self.selected_index = index;
            self.table_state.select(Some(index));
        }
    }

    fn ui(&self, f: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Header with help
                Constraint::Length(6), // Flow diagram
                Constraint::Min(15),   // Main table
                Constraint::Length(4), // Statistics and controls
            ])
            .margin(1)
            .split(f.area());

        self.render_header(f, main_layout[0]);
        self.render_flow_diagram(f, main_layout[1]);
        self.render_modern_table(f, main_layout[2]);
        self.render_footer(f, main_layout[3]);
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let title_line = Line::from(vec![
            Span::styled("⚡ ", Style::default().fg(Color::Rgb(245, 158, 11))),
            Span::styled(
                "Para Watch",
                Style::default()
                    .fg(Color::Rgb(255, 255, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " - Development Session Monitor",
                Style::default().fg(Color::Rgb(156, 163, 175)),
            ),
        ]);

        let help_line = Line::from(vec![
            Span::styled(
                "Navigation: ",
                Style::default().fg(Color::Rgb(156, 163, 175)),
            ),
            Span::styled(
                "↑↓/jk",
                Style::default()
                    .fg(Color::Rgb(99, 102, 241))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" • ", Style::default().fg(Color::Rgb(75, 85, 99))),
            Span::styled(
                "Tab",
                Style::default()
                    .fg(Color::Rgb(99, 102, 241))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " sections • ",
                Style::default().fg(Color::Rgb(156, 163, 175)),
            ),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Rgb(99, 102, 241))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " activate • ",
                Style::default().fg(Color::Rgb(156, 163, 175)),
            ),
            Span::styled(
                "q",
                Style::default()
                    .fg(Color::Rgb(239, 68, 68))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" quit", Style::default().fg(Color::Rgb(156, 163, 175))),
        ]);

        let header = Paragraph::new(vec![title_line, help_line])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(75, 85, 99)))
                    .title_style(Style::default().fg(Color::Rgb(156, 163, 175))),
            )
            .alignment(Alignment::Left);

        f.render_widget(header, area);
    }

    fn render_flow_diagram(&self, f: &mut Frame, area: Rect) {
        let working_count = self
            .sessions
            .iter()
            .filter(|s| s.state == SessionState::Working)
            .count();
        let ai_review_count = self
            .sessions
            .iter()
            .filter(|s| s.state == SessionState::AIReview)
            .count();
        let human_review_count = self
            .sessions
            .iter()
            .filter(|s| s.state == SessionState::HumanReview)
            .count();

        let current_highlight = match self.current_section {
            0 => SessionState::Working,
            1 => SessionState::AIReview,
            _ => SessionState::HumanReview,
        };

        let working_style = if current_highlight == SessionState::Working {
            Style::default()
                .fg(SessionState::Working.color())
                .bg(Color::Rgb(30, 30, 80))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(SessionState::Working.color())
                .add_modifier(Modifier::BOLD)
        };

        let ai_style = if current_highlight == SessionState::AIReview {
            Style::default()
                .fg(SessionState::AIReview.color())
                .bg(Color::Rgb(80, 60, 30))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(SessionState::AIReview.color())
                .add_modifier(Modifier::BOLD)
        };

        let human_style = if current_highlight == SessionState::HumanReview {
            Style::default()
                .fg(SessionState::HumanReview.color())
                .bg(Color::Rgb(80, 30, 60))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(SessionState::HumanReview.color())
                .add_modifier(Modifier::BOLD)
        };

        let flow_text = vec![
            Line::from(vec![
                Span::raw("        "),
                Span::styled("🔄 WORKING", working_style),
                Span::styled(" ━━━▶ ", Style::default().fg(Color::Rgb(75, 85, 99))),
                Span::styled("🤖 AI REVIEW", ai_style),
                Span::styled(" ━━━▶ ", Style::default().fg(Color::Rgb(75, 85, 99))),
                Span::styled("👤 HUMAN REVIEW", human_style),
            ]),
            Line::from(vec![
                Span::raw("          "),
                Span::styled(format!("({})", working_count), working_style),
                Span::raw("             "),
                Span::styled(format!("({})", ai_review_count), ai_style),
                Span::raw("               "),
                Span::styled(format!("({})", human_review_count), human_style),
            ]),
        ];

        let flow = Paragraph::new(flow_text)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(75, 85, 99)))
                    .title(" Workflow Pipeline ")
                    .title_style(Style::default().fg(Color::Rgb(156, 163, 175))),
            );

        f.render_widget(flow, area);
    }

    fn render_modern_table(&self, f: &mut Frame, area: Rect) {
        let header = Row::new(vec![
            Cell::from(Span::styled(
                "#",
                Style::default()
                    .fg(Color::Rgb(156, 163, 175))
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Task",
                Style::default()
                    .fg(Color::Rgb(156, 163, 175))
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Agent",
                Style::default()
                    .fg(Color::Rgb(156, 163, 175))
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Status",
                Style::default()
                    .fg(Color::Rgb(156, 163, 175))
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "State",
                Style::default()
                    .fg(Color::Rgb(156, 163, 175))
                    .add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Description",
                Style::default()
                    .fg(Color::Rgb(156, 163, 175))
                    .add_modifier(Modifier::BOLD),
            )),
        ])
        .style(Style::default().bg(Color::Rgb(17, 24, 39)))
        .height(1);

        let rows: Vec<Row> = self
            .sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let is_selected = i == self.selected_index;
                let base_style = if is_selected {
                    Style::default()
                        .bg(Color::Rgb(30, 41, 59))
                        .fg(Color::Rgb(255, 255, 255))
                } else {
                    Style::default().fg(Color::Rgb(229, 231, 235))
                };

                let state_indicator = match session.state {
                    SessionState::Working => "🔄",
                    SessionState::AIReview => "🤖",
                    SessionState::HumanReview => "👤",
                };

                let ide_status = if session.ide_open {
                    Span::styled("✓ IDE", Style::default().fg(Color::Rgb(34, 197, 94)))
                } else {
                    Span::styled("✗ ---", Style::default().fg(Color::Rgb(107, 114, 128)))
                };

                let time_display = match &session.state {
                    SessionState::AIReview => {
                        if let Some(minutes) = session.ai_review_minutes {
                            format!("⏱️ {}m", minutes)
                        } else {
                            "⏱️".to_string()
                        }
                    }
                    _ => String::new(),
                };

                let status_text = if !time_display.is_empty() {
                    format!("{} {}", ide_status, time_display)
                } else {
                    ide_status.to_string()
                };

                let state_cell = Cell::from(Span::styled(
                    format!("{} {}", state_indicator, session.state.name()),
                    Style::default()
                        .fg(session.state.color())
                        .add_modifier(Modifier::BOLD),
                ));

                let agent_display = if session.agent_name.is_empty() {
                    "-".to_string()
                } else {
                    session.agent_name.clone()
                };

                Row::new(vec![
                    Cell::from(Span::styled(format!("{}", i + 1), base_style)),
                    Cell::from(Span::styled(
                        session.task_name.clone(),
                        base_style.add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(Span::styled(agent_display, base_style)),
                    Cell::from(Text::from(status_text).style(base_style)),
                    state_cell,
                    Cell::from(Span::styled(
                        if session.description.len() > 40 {
                            format!("{}...", &session.description[..37])
                        } else {
                            session.description.clone()
                        },
                        base_style,
                    )),
                ])
                .style(base_style)
                .height(1)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(3),  // #
                Constraint::Length(15), // Task
                Constraint::Length(10), // Agent
                Constraint::Length(12), // Status
                Constraint::Length(16), // State
                Constraint::Min(20),    // Description
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(75, 85, 99)))
                .title(" Development Sessions ")
                .title_style(Style::default().fg(Color::Rgb(156, 163, 175))),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(59, 130, 246))
                .fg(Color::Rgb(255, 255, 255))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

        f.render_stateful_widget(table, area, &mut self.table_state.clone());
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Stats
                Constraint::Length(2), // Controls
            ])
            .split(area);

        // Statistics
        let stats_line = Line::from(vec![
            Span::styled("📈 Today: ", Style::default().fg(Color::Rgb(156, 163, 175))),
            Span::styled(
                "✓ 12",
                Style::default()
                    .fg(Color::Rgb(34, 197, 94))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" merged • ", Style::default().fg(Color::Rgb(156, 163, 175))),
            Span::styled(
                "✗ 3",
                Style::default()
                    .fg(Color::Rgb(239, 68, 68))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " cancelled • ",
                Style::default().fg(Color::Rgb(156, 163, 175)),
            ),
            Span::styled(
                "🔄 7",
                Style::default()
                    .fg(Color::Rgb(59, 130, 246))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " active • avg review: ",
                Style::default().fg(Color::Rgb(156, 163, 175)),
            ),
            Span::styled(
                "12m",
                Style::default()
                    .fg(Color::Rgb(245, 158, 11))
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let stats = Paragraph::new(stats_line)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::Rgb(75, 85, 99))),
            );

        // Controls hint
        let selected_task = self
            .sessions
            .get(self.selected_index)
            .map(|s| s.task_name.as_str())
            .unwrap_or("none");

        let controls_line = Line::from(vec![
            Span::styled("Selected: ", Style::default().fg(Color::Rgb(156, 163, 175))),
            Span::styled(
                selected_task,
                Style::default()
                    .fg(Color::Rgb(99, 102, 241))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" • Press ", Style::default().fg(Color::Rgb(156, 163, 175))),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Rgb(34, 197, 94))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to open IDE • ",
                Style::default().fg(Color::Rgb(156, 163, 175)),
            ),
            Span::styled(
                "Tab",
                Style::default()
                    .fg(Color::Rgb(245, 158, 11))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to switch sections",
                Style::default().fg(Color::Rgb(156, 163, 175)),
            ),
        ]);

        let controls = Paragraph::new(controls_line).alignment(Alignment::Center);

        f.render_widget(stats, layout[0]);
        f.render_widget(controls, layout[1]);
    }
}

fn create_mock_sessions() -> Vec<SessionInfo> {
    vec![
        SessionInfo {
            task_name: "auth-flow".to_string(),
            agent_name: "alice".to_string(),
            description: "OAuth2 authentication flow".to_string(),
            state: SessionState::Working,
            ide_open: true,
            ai_review_minutes: None,
        },
        SessionInfo {
            task_name: "payment-api".to_string(),
            agent_name: "bob".to_string(),
            description: "Stripe payment integration".to_string(),
            state: SessionState::Working,
            ide_open: true,
            ai_review_minutes: None,
        },
        SessionInfo {
            task_name: "email-svc".to_string(),
            agent_name: "charlie".to_string(),
            description: "Email notification service".to_string(),
            state: SessionState::Working,
            ide_open: false,
            ai_review_minutes: None,
        },
        SessionInfo {
            task_name: "search".to_string(),
            agent_name: "dave".to_string(),
            description: "Elasticsearch integration".to_string(),
            state: SessionState::Working,
            ide_open: true,
            ai_review_minutes: None,
        },
        SessionInfo {
            task_name: "ui-components".to_string(),
            agent_name: "eve".to_string(),
            description: "React dashboard components".to_string(),
            state: SessionState::AIReview,
            ide_open: false,
            ai_review_minutes: Some(15),
        },
        SessionInfo {
            task_name: "api-tests".to_string(),
            agent_name: "frank".to_string(),
            description: "API integration test suite".to_string(),
            state: SessionState::AIReview,
            ide_open: false,
            ai_review_minutes: Some(8),
        },
        SessionInfo {
            task_name: "backend-api".to_string(),
            agent_name: "".to_string(),
            description: "REST API implementation".to_string(),
            state: SessionState::HumanReview,
            ide_open: false,
            ai_review_minutes: None,
        },
    ]
}

fn load_real_sessions(config: &crate::config::Config) -> Result<Vec<SessionInfo>> {
    use crate::core::session::{SessionManager, SessionStatus};
    use crate::utils::names::generate_unique_name;

    let session_manager = SessionManager::new(config);
    let sessions = session_manager.list_sessions()?;

    let mut session_infos = Vec::new();
    let used_names = Vec::new(); // For future use with better agent naming

    for session in sessions {
        // Determine session state based on status and worktree existence
        let state = if session.status == SessionStatus::Active && session.worktree_path.exists() {
            SessionState::Working
        } else if session.status == SessionStatus::Finished
            || (session.status == SessionStatus::Active && !session.worktree_path.exists())
        {
            SessionState::HumanReview
        } else {
            // Cancelled sessions are not shown in the TUI for now
            continue;
        };

        // Generate agent name (simplified for now)
        let agent_name = generate_unique_name(&used_names);

        // Extract description or use default
        let description = session
            .description
            .unwrap_or_else(|| "No description provided".to_string());

        session_infos.push(SessionInfo {
            task_name: session.name.clone(),
            agent_name,
            description,
            state,
            ide_open: state == SessionState::Working, // Working sessions have IDE open
            ai_review_minutes: None,                  // AI review not implemented yet
        });
    }

    Ok(session_infos)
}

fn load_sessions_with_fallback(config: &crate::config::Config) -> Vec<SessionInfo> {
    match load_real_sessions(config) {
        Ok(mut real_sessions) => {
            // Add some mock AI Review sessions for demonstration
            real_sessions.extend(vec![
                SessionInfo {
                    task_name: "ui-components".to_string(),
                    agent_name: "ai-eve".to_string(),
                    description: "React dashboard components (mock)".to_string(),
                    state: SessionState::AIReview,
                    ide_open: false,
                    ai_review_minutes: Some(15),
                },
                SessionInfo {
                    task_name: "api-tests".to_string(),
                    agent_name: "ai-frank".to_string(),
                    description: "API integration test suite (mock)".to_string(),
                    state: SessionState::AIReview,
                    ide_open: false,
                    ai_review_minutes: Some(8),
                },
            ]);
            real_sessions
        }
        Err(e) => {
            eprintln!(
                "Warning: Failed to load real session data: {}. Using mock data.",
                e
            );
            create_mock_sessions()
        }
    }
}

pub fn execute(_args: WatchArgs) -> Result<()> {
    let config = crate::config::Config::load_or_create()
        .map_err(|e| crate::utils::ParaError::config_error(e.to_string()))?;
    let mut app = App::with_real_data(&config);
    app.run()
        .map_err(|e| crate::utils::ParaError::watch_error(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::{SessionManager, SessionState as CoreSessionState, SessionStatus};
    use tempfile::TempDir;

    fn create_simple_test_config(state_dir: &str) -> crate::config::Config {
        crate::config::Config {
            ide: crate::config::IdeConfig {
                name: "echo".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: crate::config::WrapperConfig {
                    enabled: false,
                    name: "".to_string(),
                    command: "".to_string(),
                },
            },
            directories: crate::config::DirectoryConfig {
                state_dir: state_dir.to_string(),
                subtrees_dir: ".para/worktrees".to_string(),
            },
            git: crate::config::defaults::default_git_config(),
            session: crate::config::defaults::default_session_config(),
        }
    }

    #[test]
    fn test_load_real_sessions_empty() {
        let temp_dir = TempDir::new().unwrap();
        let config =
            create_simple_test_config(&temp_dir.path().join(".para_state").to_string_lossy());

        let sessions = load_real_sessions(&config).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_load_real_sessions_with_active_worktree() {
        let temp_dir = TempDir::new().unwrap();
        let config =
            create_simple_test_config(&temp_dir.path().join(".para_state").to_string_lossy());

        // Create session manager
        let session_manager = SessionManager::new(&config);

        // Create a test session with worktree
        let session_state = CoreSessionState::with_description(
            "test-session".to_string(),
            "para/test-session".to_string(),
            temp_dir.path().join(".para/worktrees/test-session"),
            Some("Test description".to_string()),
        );

        // Create the worktree directory to simulate active session
        std::fs::create_dir_all(&session_state.worktree_path).unwrap();

        session_manager.save_state(&session_state).unwrap();

        let sessions = load_real_sessions(&config).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].task_name, "test-session");
        assert_eq!(sessions[0].description, "Test description");
        assert!(matches!(sessions[0].state, SessionState::Working));
    }

    #[test]
    fn test_load_real_sessions_finished_no_worktree() {
        let temp_dir = TempDir::new().unwrap();
        let config =
            create_simple_test_config(&temp_dir.path().join(".para_state").to_string_lossy());

        // Create session manager
        let session_manager = SessionManager::new(&config);

        // Create a finished session without worktree
        let mut session_state = CoreSessionState::with_description(
            "finished-session".to_string(),
            "para/finished-session".to_string(),
            temp_dir.path().join(".para/worktrees/finished-session"),
            Some("Finished task".to_string()),
        );
        session_state.update_status(SessionStatus::Finished);

        // Don't create worktree directory to simulate finished session
        session_manager.save_state(&session_state).unwrap();

        let sessions = load_real_sessions(&config).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].task_name, "finished-session");
        assert_eq!(sessions[0].description, "Finished task");
        assert!(matches!(sessions[0].state, SessionState::HumanReview));
    }
}
