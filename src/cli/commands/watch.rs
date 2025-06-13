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
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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

#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Working,
    AIReview,
    HumanReview,
}

impl SessionState {
    fn color(&self) -> Color {
        match self {
            SessionState::Working => Color::Blue,
            SessionState::AIReview => Color::Yellow,
            SessionState::HumanReview => Color::Magenta,
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
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            sessions: create_mock_sessions(),
            selected_index: 0,
            should_quit: false,
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
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('r') => println!("Refreshing..."),
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let num = c.to_digit(10).unwrap() as usize;
                if num > 0 && num <= self.sessions.len() {
                    let session = &self.sessions[num - 1];
                    println!("Opening IDE for task: {}", session.task_name);
                }
            }
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_index < self.sessions.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            _ => {}
        }
    }

    fn ui(&self, f: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(5), // Flow diagram
                Constraint::Min(10),   // Task lists
                Constraint::Length(3), // Statistics
            ])
            .split(f.area());

        self.render_header(f, main_layout[0]);
        self.render_flow_diagram(f, main_layout[1]);
        self.render_task_lists(f, main_layout[2]);
        self.render_statistics(f, main_layout[3]);
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let header = Paragraph::new("💻 IDE Quick Switcher")
            .style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Para Watch")
                    .title_alignment(Alignment::Left)
                    .border_style(Style::default().fg(Color::Blue)),
            );

        let controls = Paragraph::new("[q]uit [r]efresh")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Right);

        let header_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area.inner(Margin::new(1, 0)));

        f.render_widget(header, area);
        f.render_widget(controls, header_layout[1]);
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

        let flow_text = vec![
            Line::from(vec![
                Span::raw("     "),
                Span::styled(
                    "WORKING",
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" ─────▶ "),
                Span::styled(
                    "AI REVIEW",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" ─────▶ "),
                Span::styled(
                    "HUMAN REVIEW",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::raw("       "),
                Span::styled(
                    format!("({})", working_count),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw("            "),
                Span::styled(
                    format!("({})", ai_review_count),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw("              "),
                Span::styled(
                    format!("({})", human_review_count),
                    Style::default().fg(Color::Magenta),
                ),
            ]),
        ];

        let flow = Paragraph::new(flow_text)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray)),
            );

        f.render_widget(flow, area);
    }

    fn render_task_lists(&self, f: &mut Frame, area: Rect) {
        let mut items = Vec::new();
        let mut current_state = None;
        let mut task_number = 1;

        for session in &self.sessions {
            if current_state != Some(&session.state) {
                if !items.is_empty() {
                    items.push(ListItem::new(""));
                }

                let count = self
                    .sessions
                    .iter()
                    .filter(|s| s.state == session.state)
                    .count();
                let state_header =
                    format!("{} ({}) {}", session.state.name(), count, "─".repeat(60));
                items.push(
                    ListItem::new(state_header).style(
                        Style::default()
                            .fg(session.state.color())
                            .add_modifier(Modifier::BOLD),
                    ),
                );
                current_state = Some(&session.state);
            }

            let ide_status = if session.ide_open { "✓" } else { "✗" };
            let time_info = match &session.state {
                SessionState::AIReview => {
                    if let Some(minutes) = session.ai_review_minutes {
                        format!("⏱️ {}m", minutes)
                    } else {
                        "⏱️".to_string()
                    }
                }
                SessionState::HumanReview => "📝".to_string(),
                _ => ide_status.to_string(),
            };

            let task_line = format!(
                "  [{}] {:12} 👤 {:8} {:6} \"{}\"",
                task_number, session.task_name, session.agent_name, time_info, session.description
            );

            items.push(ListItem::new(task_line).style(Style::default().fg(Color::White)));

            task_number += 1;
        }

        let tasks = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray)),
        );

        f.render_widget(tasks, area);
    }

    fn render_statistics(&self, f: &mut Frame, area: Rect) {
        let stats_text = Line::from(vec![
            Span::raw("Today: "),
            Span::styled("✅ 12 Merged", Style::default().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled("❌ 3 Cancelled", Style::default().fg(Color::Red)),
            Span::raw(" | "),
            Span::styled("🔄 7 Active", Style::default().fg(Color::Blue)),
        ]);

        let stats = Paragraph::new(stats_text)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray)),
            );

        f.render_widget(stats, area);
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

pub fn execute(_args: WatchArgs) -> Result<()> {
    let mut app = App::new();
    app.run()
        .map_err(|e| crate::utils::ParaError::watch_error(e.to_string()))
}
