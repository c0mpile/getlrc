pub mod state;
pub mod ui;
pub mod widgets;

use crate::messages::{UiMessage, WorkerMessage};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use state::AppState;
use std::io;
use tokio::sync::mpsc;

pub struct App {
    state: AppState,
    worker_rx: mpsc::UnboundedReceiver<WorkerMessage>,
    ui_tx: mpsc::UnboundedSender<UiMessage>,
}

impl App {
    pub fn new(
        worker_rx: mpsc::UnboundedReceiver<WorkerMessage>,
        ui_tx: mpsc::UnboundedSender<UiMessage>,
    ) -> Self {
        Self {
            state: AppState::new(),
            worker_rx,
            ui_tx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_loop(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    async fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        loop {
            // Render UI
            terminal.draw(|f| ui::render(f, &self.state))?;

            // Handle events (non-blocking)
            if event::poll(std::time::Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            self.state.should_quit = true;
                            let _ = self.ui_tx.send(UiMessage::Quit);
                            break;
                        }
                        KeyCode::Char('p') => {
                            if !self.state.paused {
                                self.state.paused = true;
                                let _ = self.ui_tx.send(UiMessage::Pause);
                            }
                        }
                        KeyCode::Char('r') => {
                            if self.state.paused {
                                self.state.paused = false;
                                let _ = self.ui_tx.send(UiMessage::Resume);
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Process worker messages (non-blocking)
            while let Ok(msg) = self.worker_rx.try_recv() {
                self.state.update(msg);
            }

            // Exit if worker is done and user hasn't quit
            if self.state.status == state::Status::Complete && self.state.should_quit {
                break;
            }

            // 60fps target
            tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        }

        Ok(())
    }
}
