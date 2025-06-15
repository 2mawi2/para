use crate::cli::parser::MonitorArgs;
use crate::ui::monitor::MonitorCoordinator;
use crate::utils::Result;
use anyhow::Result as AnyhowResult;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;

pub struct App {
    coordinator: MonitorCoordinator,
}

impl App {
    pub fn new(config: crate::config::Config) -> Self {
        Self {
            coordinator: MonitorCoordinator::new(config),
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
        // Initial draw
        terminal.draw(|f| self.coordinator.render(f))?;

        loop {
            // Check if we need to refresh sessions
            let should_refresh = self.coordinator.should_refresh();
            if should_refresh {
                self.coordinator.refresh_sessions();
                self.coordinator.mark_refreshed();
            }

            // Poll for events with timeout for refresh
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.coordinator.handle_key(key).unwrap_or(());

                    if self.coordinator.should_quit() {
                        break;
                    }

                    // Redraw after handling key event
                    terminal.draw(|f| self.coordinator.render(f))?;
                }
            } else if should_refresh {
                // Only redraw if we refreshed sessions
                terminal.draw(|f| self.coordinator.render(f))?;
            }
        }
        Ok(())
    }
}

pub fn execute(_args: MonitorArgs) -> Result<()> {
    let config = crate::config::Config::load_or_create()
        .map_err(|e| crate::utils::ParaError::config_error(e.to_string()))?;
    let mut app = App::new(config);
    app.run()
        .map_err(|e| crate::utils::ParaError::ide_error(format!("Monitor UI error: {}", e)))
}
