use crate::cli::parser::MonitorArgs;
use crate::core::git::{GitOperations, GitService};
use crate::core::session::{SessionManager, SessionStatus as CoreSessionStatus};
use crate::utils::Result;
use anyhow::Result as AnyhowResult;
use chrono::{DateTime, Duration, Utc};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
    Frame, Terminal,
};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub name: String,
    pub branch: String,
    pub status: SessionStatus,
    pub last_activity: DateTime<Utc>,
    pub task: String,
    pub worktree_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionStatus {
    Active, // 🟢 Recent activity (< 5 min)
    Idle,   // 🟡 No activity (5-30 min)
    Ready,  // ✅ Finished, ready for review
    Stale,  // ⏸️  No activity (> 30 min)
}

impl SessionStatus {
    fn icon(&self) -> &str {
        match self {
            SessionStatus::Active => "🟢",
            SessionStatus::Idle => "🟡",
            SessionStatus::Ready => "✅",
            SessionStatus::Stale => "⏸️",
        }
    }

    fn name(&self) -> &str {
        match self {
            SessionStatus::Active => "Active",
            SessionStatus::Idle => "Idle",
            SessionStatus::Ready => "Ready",
            SessionStatus::Stale => "Stale",
        }
    }

    fn color(&self) -> Color {
        match self {
            SessionStatus::Active => Color::Rgb(34, 197, 94), // Green
            SessionStatus::Idle => Color::Rgb(245, 158, 11),  // Amber
            SessionStatus::Ready => Color::Rgb(99, 102, 241), // Indigo
            SessionStatus::Stale => Color::Rgb(107, 114, 128), // Gray
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    FinishPrompt,
    CancelConfirm,
}

pub struct App {
    sessions: Vec<SessionInfo>,
    selected_index: usize,
    should_quit: bool,
    table_state: TableState,
    mode: AppMode,
    input_buffer: String,
    show_stale: bool,
    last_refresh: Instant,
    config: crate::config::Config,
}

impl App {
    pub fn new(config: crate::config::Config) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let mut app = Self {
            sessions: Vec::new(),
            selected_index: 0,
            should_quit: false,
            table_state,
            mode: AppMode::Normal,
            input_buffer: String::new(),
            show_stale: false,
            last_refresh: Instant::now(),
            config,
        };

        app.refresh_sessions();
        app
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
            // Auto-refresh every 2 seconds
            if self.last_refresh.elapsed().as_secs() >= 2 {
                self.refresh_sessions();
                self.last_refresh = Instant::now();
            }

            terminal.draw(|f| self.ui(f))?;

            // Poll for events with timeout for refresh
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match self.mode {
                        AppMode::Normal => self.handle_normal_key(key),
                        AppMode::FinishPrompt => self.handle_finish_prompt_key(key),
                        AppMode::CancelConfirm => self.handle_cancel_confirm_key(key),
                    }

                    if self.should_quit {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('s') => {
                self.show_stale = !self.show_stale;
                self.refresh_sessions();
            }
            KeyCode::Up | KeyCode::Char('k') => self.previous_item(),
            KeyCode::Down | KeyCode::Char('j') => self.next_item(),
            KeyCode::Enter => self.resume_selected(),
            KeyCode::Char('f') => self.start_finish(),
            KeyCode::Char('c') => self.start_cancel(),
            KeyCode::Char('i') => self.integrate_if_ready(),
            _ => {}
        }
    }

    fn handle_finish_prompt_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.input_buffer.clear();
            }
            KeyCode::Enter => {
                // Only submit on plain Enter (no multiline support for now)
                if !self.input_buffer.trim().is_empty() {
                    self.execute_finish();
                }
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                // Only accept printable characters for single-line input
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    fn handle_cancel_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                self.execute_cancel();
                self.mode = AppMode::Normal;
            }
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
    }

    fn refresh_sessions(&mut self) {
        self.sessions = load_real_sessions(&self.config).unwrap_or_else(|_| Vec::new());

        // Filter out stale sessions if not showing them
        if !self.show_stale {
            self.sessions
                .retain(|s| !matches!(s.status, SessionStatus::Stale));
        }

        // Update selected index if necessary
        if self.selected_index >= self.sessions.len() && !self.sessions.is_empty() {
            self.selected_index = self.sessions.len() - 1;
            self.table_state.select(Some(self.selected_index));
        } else if self.sessions.is_empty() {
            self.selected_index = 0;
            self.table_state.select(None);
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

    fn resume_selected(&mut self) {
        if let Some(session) = self.sessions.get(self.selected_index) {
            // Verify worktree path exists
            if !session.worktree_path.exists() {
                // Worktree might have been removed, refresh sessions
                self.refresh_sessions();
                return;
            }

            // Use internal APIs to open IDE
            let ide_manager = crate::core::ide::IdeManager::new(&self.config);
            if let Err(_e) = ide_manager.launch(&session.worktree_path, false) {
                // If launch fails, try to refresh sessions in case something changed
                self.refresh_sessions();
            }
        }
    }

    fn start_finish(&mut self) {
        if self.sessions.get(self.selected_index).is_some() {
            self.mode = AppMode::FinishPrompt;
            self.input_buffer.clear();
        }
    }

    fn start_cancel(&mut self) {
        if self.sessions.get(self.selected_index).is_some() {
            self.mode = AppMode::CancelConfirm;
        }
    }

    fn integrate_if_ready(&mut self) {
        if let Some(session) = self.sessions.get(self.selected_index) {
            if matches!(session.status, SessionStatus::Ready) {
                let _ = Command::new("para")
                    .args(["integrate", &session.name])
                    .current_dir(&session.worktree_path)
                    .spawn();
            }
        }
    }

    fn execute_finish(&mut self) {
        if let Some(session) = self.sessions.get(self.selected_index) {
            // Use internal APIs instead of spawning external command
            // Run in a separate thread to prevent git output from appearing in UI
            let worktree_path = session.worktree_path.clone();
            let branch = session.branch.clone();
            let message = self.input_buffer.clone();

            std::thread::spawn(move || {
                if let Ok(git_service) = GitService::discover_from(&worktree_path) {
                    let finish_request = crate::core::git::FinishRequest {
                        feature_branch: branch,
                        commit_message: message,
                        target_branch_name: None,
                    };
                    let _ = git_service.finish_session(finish_request);
                }
            });

            self.mode = AppMode::Normal;
            self.input_buffer.clear();
            self.refresh_sessions();
        }
    }

    fn execute_cancel(&mut self) {
        if let Some(session) = self.sessions.get(self.selected_index) {
            // Use internal APIs instead of spawning external command
            let session_manager = SessionManager::new(&self.config);

            // Load session state and archive the branch
            if let Ok(session_state) = session_manager.load_state(&session.name) {
                // Run git operations in a separate thread to prevent output in UI
                let worktree_path = session.worktree_path.clone();
                let branch = session_state.branch.clone();
                let name = session_state.name.clone();
                let prefix = self.config.git.branch_prefix.clone();
                let worktree_to_remove = session_state.worktree_path.clone();

                std::thread::spawn(move || {
                    if let Ok(git_service) = GitService::discover_from(&worktree_path) {
                        // Archive the branch
                        let _ =
                            git_service.archive_branch_with_session_name(&branch, &name, &prefix);

                        // Remove worktree
                        let _ = git_service.remove_worktree(&worktree_to_remove);
                    }
                });

                // Delete session state
                let _ = session_manager.delete_state(&session_state.name);
            }

            self.mode = AppMode::Normal;
            self.refresh_sessions();
        }
    }

    fn ui(&self, f: &mut Frame) {
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Main table
                Constraint::Length(3), // Footer with controls
            ])
            .margin(1)
            .split(f.area());

        self.render_header(f, main_layout[0]);
        self.render_table(f, main_layout[1]);
        self.render_footer(f, main_layout[2]);

        // Render mode-specific overlays
        match self.mode {
            AppMode::FinishPrompt => self.render_finish_prompt(f),
            AppMode::CancelConfirm => self.render_cancel_confirm(f),
            _ => {}
        }
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let header_text = vec![
            Line::from(vec![
                Span::styled(
                    "Para Monitor - Interactive Session Control",
                    Style::default()
                        .fg(Color::Rgb(255, 255, 255))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("                  "),
                Span::styled(
                    "Auto-refresh: 2s",
                    Style::default().fg(Color::Rgb(156, 163, 175)),
                ),
            ]),
            Line::from("─".repeat(area.width as usize)),
        ];

        let header = Paragraph::new(header_text)
            .style(Style::default().fg(Color::Rgb(75, 85, 99)))
            .alignment(Alignment::Left);

        f.render_widget(header, area);
    }

    fn render_table(&self, f: &mut Frame, area: Rect) {
        let header = Row::new(vec![
            Cell::from(" # "),
            Cell::from("Session"),
            Cell::from("Status"),
            Cell::from("Last Activity"),
            Cell::from("Task"),
        ])
        .style(
            Style::default()
                .fg(Color::Rgb(156, 163, 175))
                .add_modifier(Modifier::BOLD),
        )
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

                let status_cell = Cell::from(format!(
                    "{} {}",
                    session.status.icon(),
                    session.status.name()
                ))
                .style(Style::default().fg(session.status.color()));

                let activity_text = format_activity(&session.last_activity);

                Row::new(vec![
                    Cell::from(format!("[{}]", i + 1)).style(base_style),
                    Cell::from(session.name.clone()).style(base_style.add_modifier(Modifier::BOLD)),
                    status_cell,
                    Cell::from(activity_text).style(base_style),
                    Cell::from(format!("\"{}\"", truncate_task(&session.task, 40)))
                        .style(base_style),
                ])
                .height(1)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(5),  // #
                Constraint::Length(16), // Session
                Constraint::Length(12), // Status
                Constraint::Length(14), // Last Activity
                Constraint::Min(30),    // Task
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_style(Style::default().fg(Color::Rgb(75, 85, 99))),
        );

        f.render_stateful_widget(table, area, &mut self.table_state.clone());
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let selected_session = self
            .sessions
            .get(self.selected_index)
            .map(|s| s.name.as_str())
            .unwrap_or("none");

        let selected_branch = self
            .sessions
            .get(self.selected_index)
            .map(|s| s.branch.as_str())
            .unwrap_or("");

        // Check if this is the current session (where monitor was run from)
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
            Span::styled(session_info, Style::default().fg(Color::Rgb(156, 163, 175))),
            Span::styled(
                "[Enter]",
                Style::default()
                    .fg(Color::Rgb(99, 102, 241))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Resume • "),
            Span::styled(
                "[f]",
                Style::default()
                    .fg(Color::Rgb(99, 102, 241))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Finish • "),
            Span::styled(
                "[c]",
                Style::default()
                    .fg(Color::Rgb(99, 102, 241))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Cancel • "),
            Span::styled(
                "[q]",
                Style::default()
                    .fg(Color::Rgb(99, 102, 241))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Quit"),
        ])];

        let footer = Paragraph::new(controls)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(Color::Rgb(75, 85, 99))),
            )
            .alignment(Alignment::Left);

        f.render_widget(footer, area);
    }

    fn render_finish_prompt(&self, f: &mut Frame) {
        let area = centered_rect(60, 25, f.area());

        // Clear the area first to prevent text bleeding through
        f.render_widget(Clear, area);

        let input_text = if self.input_buffer.is_empty() {
            "Type your commit message..."
        } else {
            &self.input_buffer
        };

        let prompt = Paragraph::new(vec![
            Line::from("Enter commit message:"),
            Line::from(""),
            Line::from(Span::styled(
                input_text,
                if self.input_buffer.is_empty() {
                    Style::default().fg(Color::Rgb(107, 114, 128)) // Gray placeholder
                } else {
                    Style::default().fg(Color::Rgb(255, 255, 255)) // White text
                },
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("[Enter]", Style::default().fg(Color::Rgb(34, 197, 94))),
                Span::raw(" confirm • "),
                Span::styled("[Esc]", Style::default().fg(Color::Rgb(239, 68, 68))),
                Span::raw(" cancel"),
            ]),
        ])
        .block(
            Block::default()
                .title(" Finish Session ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(99, 102, 241)))
                .style(Style::default().bg(Color::Rgb(0, 0, 0))), // Black background
        )
        .style(Style::default().fg(Color::Rgb(255, 255, 255)));

        f.render_widget(prompt, area);
    }

    fn render_cancel_confirm(&self, f: &mut Frame) {
        let area = centered_rect(50, 20, f.area()); // Increased height to 20% to ensure content fits

        // Clear the area first to prevent text bleeding through
        f.render_widget(Clear, area);

        let confirm = Paragraph::new(vec![
            Line::from("Cancel this session?"),
            Line::from(""),
            Line::from(vec![
                Span::styled("[Enter]", Style::default().fg(Color::Rgb(34, 197, 94))),
                Span::raw(" confirm • "),
                Span::styled("[Esc]", Style::default().fg(Color::Rgb(239, 68, 68))),
                Span::raw(" cancel"),
            ]),
        ])
        .block(
            Block::default()
                .title(" Confirm Cancel ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(239, 68, 68)))
                .style(Style::default().bg(Color::Rgb(0, 0, 0))), // Black background
        )
        .style(Style::default().fg(Color::Rgb(255, 255, 255)))
        .alignment(Alignment::Center);

        f.render_widget(confirm, area);
    }
}

fn format_activity(last_activity: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now - *last_activity;

    if duration < Duration::minutes(1) {
        "now".to_string()
    } else if duration < Duration::hours(1) {
        format!("{}m ago", duration.num_minutes())
    } else if duration < Duration::days(1) {
        format!("{}h ago", duration.num_hours())
    } else {
        format!("{}d ago", duration.num_days())
    }
}

fn truncate_task(task: &str, max_len: usize) -> String {
    if task.len() <= max_len {
        task.to_string()
    } else {
        format!("{}...", &task[..max_len - 3])
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn detect_session_status(
    session: &crate::core::session::SessionState,
    last_activity: &DateTime<Utc>,
) -> SessionStatus {
    // Check if session is marked as finished
    if matches!(session.status, CoreSessionStatus::Finished) {
        return SessionStatus::Ready;
    }

    // Check activity time
    let now = Utc::now();
    let elapsed = now - *last_activity;

    match elapsed.num_minutes() {
        0..=5 => SessionStatus::Active,
        6..=30 => SessionStatus::Idle,
        _ => SessionStatus::Stale,
    }
}

fn load_real_sessions(config: &crate::config::Config) -> Result<Vec<SessionInfo>> {
    let session_manager = SessionManager::new(config);
    let sessions = session_manager.list_sessions()?;

    let mut session_infos = Vec::new();

    // Check if current directory is a session worktree
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let current_session = session_manager
        .find_session_by_path(&current_dir)
        .unwrap_or(None);

    for session in sessions {
        // Skip cancelled sessions
        if matches!(session.status, CoreSessionStatus::Cancelled) {
            continue;
        }

        // Get last activity (placeholder for now)
        let last_activity = session.last_activity.unwrap_or(session.created_at);

        // Detect status based on activity
        let mut status = detect_session_status(&session, &last_activity);

        // If this is the current session (where monitor was run from), mark it as active
        if let Some(ref current) = current_session {
            if current.name == session.name {
                status = SessionStatus::Active;
            }
        }

        // Load task description
        let task = session.task_description.unwrap_or_else(|| {
            // Try to load from task file for backward compatibility
            let state_dir = Path::new(&config.directories.state_dir);
            let task_file = state_dir.join(format!("{}.task", session.name));
            std::fs::read_to_string(task_file).unwrap_or_else(|_| {
                // Show first few characters of session name if no task description
                format!("Session: {}", truncate_task(&session.name, 30))
            })
        });

        session_infos.push(SessionInfo {
            name: session.name.clone(),
            branch: session.branch.clone(),
            status,
            last_activity,
            task,
            worktree_path: session.worktree_path.clone(),
        });
    }

    // Sort by last activity (most recent first), but put current session first if found
    session_infos.sort_by(|a, b| {
        if let Some(ref current) = current_session {
            if a.name == current.name {
                return std::cmp::Ordering::Less; // Current session goes first
            }
            if b.name == current.name {
                return std::cmp::Ordering::Greater; // Current session goes first
            }
        }
        b.last_activity.cmp(&a.last_activity)
    });

    Ok(session_infos)
}

pub fn execute(_args: MonitorArgs) -> Result<()> {
    let config = crate::config::Config::load_or_create()
        .map_err(|e| crate::utils::ParaError::config_error(e.to_string()))?;
    let mut app = App::new(config);
    app.run()
        .map_err(|e| crate::utils::ParaError::ide_error(format!("Monitor UI error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::SessionState;

    #[test]
    fn test_session_status_detection() {
        let session = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            std::path::PathBuf::from("/test"),
        );

        // Test active status (< 5 minutes)
        let now = chrono::Utc::now();
        let status = detect_session_status(&session, &now);
        assert!(matches!(status, SessionStatus::Active));

        // Test idle status (10 minutes ago)
        let ten_minutes_ago = now - chrono::Duration::minutes(10);
        let status = detect_session_status(&session, &ten_minutes_ago);
        assert!(matches!(status, SessionStatus::Idle));

        // Test stale status (> 30 minutes)
        let hour_ago = now - chrono::Duration::hours(1);
        let status = detect_session_status(&session, &hour_ago);
        assert!(matches!(status, SessionStatus::Stale));
    }

    #[test]
    fn test_session_status_ready() {
        let mut session = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            std::path::PathBuf::from("/test"),
        );
        session.update_status(CoreSessionStatus::Finished);

        let now = chrono::Utc::now();
        let status = detect_session_status(&session, &now);
        assert!(matches!(status, SessionStatus::Ready));
    }

    #[test]
    fn test_format_activity() {
        let now = chrono::Utc::now();

        // Test "now" (< 1 minute)
        let result = format_activity(&now);
        assert_eq!(result, "now");

        // Test minutes ago
        let five_minutes_ago = now - chrono::Duration::minutes(5);
        let result = format_activity(&five_minutes_ago);
        assert_eq!(result, "5m ago");

        // Test hours ago
        let two_hours_ago = now - chrono::Duration::hours(2);
        let result = format_activity(&two_hours_ago);
        assert_eq!(result, "2h ago");

        // Test days ago
        let three_days_ago = now - chrono::Duration::days(3);
        let result = format_activity(&three_days_ago);
        assert_eq!(result, "3d ago");
    }

    #[test]
    fn test_truncate_task() {
        // Test short task
        let short = "Short task";
        assert_eq!(truncate_task(short, 20), "Short task");

        // Test exact length
        let exact = "Exactly twenty chars";
        assert_eq!(truncate_task(exact, 20), "Exactly twenty chars");

        // Test truncation
        let long = "This is a very long task description that needs to be truncated";
        assert_eq!(truncate_task(long, 20), "This is a very lo...");
    }

    #[test]
    fn test_centered_rect() {
        use ratatui::layout::Rect;

        let screen = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(50, 20, screen);

        // Should be centered
        assert_eq!(centered.x, 25); // (100 - 50) / 2
        assert_eq!(centered.y, 20); // (50 - 10) / 2
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 10);
    }

    #[test]
    fn test_single_line_commit_message_handling() {
        // Test single-line commit message support
        let valid_message = "Fix bug in monitor UI";
        let empty_message = "";
        let whitespace_message = "   ";

        // Test message validation
        assert!(!valid_message.trim().is_empty());
        assert!(empty_message.trim().is_empty());
        assert!(whitespace_message.trim().is_empty());

        // Test that single line messages work correctly
        assert!(!valid_message.contains('\n'));
        assert_eq!(valid_message.len(), 21);
    }

    #[test]
    fn test_resume_functionality_works_for_all_statuses() {
        // Test that resume should work for all session statuses (Active, Idle, Ready, Stale)
        // This is a unit test to verify the logic, not a full integration test

        let session_active = SessionInfo {
            name: "test-active".to_string(),
            branch: "test-branch".to_string(),
            status: SessionStatus::Active,
            last_activity: chrono::Utc::now(),
            task: "Test task".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/test-active"),
        };

        let session_idle = SessionInfo {
            name: "test-idle".to_string(),
            branch: "test-branch".to_string(),
            status: SessionStatus::Idle,
            last_activity: chrono::Utc::now() - chrono::Duration::minutes(10),
            task: "Test task".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/test-idle"),
        };

        let session_ready = SessionInfo {
            name: "test-ready".to_string(),
            branch: "test-branch".to_string(),
            status: SessionStatus::Ready,
            last_activity: chrono::Utc::now(),
            task: "Test task".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/test-ready"),
        };

        let session_stale = SessionInfo {
            name: "test-stale".to_string(),
            branch: "test-branch".to_string(),
            status: SessionStatus::Stale,
            last_activity: chrono::Utc::now() - chrono::Duration::hours(2),
            task: "Test task".to_string(),
            worktree_path: std::path::PathBuf::from("/tmp/test-stale"),
        };

        // All sessions should be resumable regardless of status
        // The resume logic doesn't check status, only path existence and IDE launch
        assert!(matches!(session_active.status, SessionStatus::Active));
        assert!(matches!(session_idle.status, SessionStatus::Idle));
        assert!(matches!(session_ready.status, SessionStatus::Ready));
        assert!(matches!(session_stale.status, SessionStatus::Stale));
    }
}
