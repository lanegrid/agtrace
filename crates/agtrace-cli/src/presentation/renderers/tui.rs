//! TUI Renderer for Watch command
//!
//! This module implements the TUI event loop and screen rendering.
//! It receives `TuiScreenViewModel` updates via channel and renders them using Ratatui.
//!
//! ## Architecture: Multi-Page TUI with State/Data Separation
//!
//! ### Core Principles:
//! 1. **Data (ViewModel)** = Read-only snapshot from Presenter
//! 2. **State (UI State)** = Mutable UI context (scroll, selection, active page)
//! 3. **Renderer** = Router that delegates to page-specific handlers
//! 4. **Index Safety** = Always verify cursor positions against data bounds
//!
//! ### Responsibilities:
//! - Renderer owns UI state (scroll positions, selected items, active page)
//! - Renderer does NOT own data (receives ViewModels via channel)
//! - Uses View widgets to render the screen
//! - Handles keyboard input for navigation and control
//! - Delegates page-specific logic to handler methods

use std::io;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};

use crate::presentation::view_models::TuiScreenViewModel;
use crate::presentation::views::tui::components::DashboardComponent;

/// TUI events sent from handler to renderer
pub enum TuiEvent {
    /// Update screen with new ViewModel
    Update(Box<TuiScreenViewModel>),
    /// Fatal error occurred
    Error(String),
    /// Notification message (non-fatal info)
    Notification(String),
}

/// Signal sent from renderer to handler
pub enum RendererSignal {
    /// User requested quit
    Quit,
}

/// TUI Renderer application state
///
/// Acts as a Router/Orchestrator that delegates to Components.
/// Separates immutable data (ViewModel) from mutable UI state (Components).
pub struct TuiRenderer {
    /// Current screen data (received from handler) - IMMUTABLE DATA
    current_screen: Option<TuiScreenViewModel>,

    /// Dashboard component (owns UI state + input + render logic) - COMPONENT
    dashboard_component: DashboardComponent,

    /// Global quit flag - UI STATE
    should_quit: bool,

    /// Error message to display (if any) - UI STATE
    error_message: Option<String>,

    /// Notification message (non-fatal info) - UI STATE
    notification_message: Option<String>,

    /// Sender to notify handler of renderer events
    signal_tx: Option<Sender<RendererSignal>>,
}

impl TuiRenderer {
    pub fn new() -> Self {
        Self {
            current_screen: None,
            dashboard_component: DashboardComponent::new(),
            should_quit: false,
            error_message: None,
            notification_message: None,
            signal_tx: None,
        }
    }

    pub fn with_signal_sender(mut self, tx: Sender<RendererSignal>) -> Self {
        self.signal_tx = Some(tx);
        self
    }

    /// Main event loop for TUI rendering
    ///
    /// This function:
    /// 1. Sets up terminal in raw mode
    /// 2. Receives ViewModel updates via channel
    /// 3. Handles keyboard input
    /// 4. Renders the screen using Views
    /// 5. Cleans up terminal on exit
    pub fn run(mut self, rx: Receiver<TuiEvent>) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run event loop
        let result = self.event_loop(&mut terminal, rx);

        // Cleanup terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    /// Main event loop
    fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        rx: Receiver<TuiEvent>,
    ) -> Result<()> {
        loop {
            // Draw current state
            terminal.draw(|f| self.render(f))?;

            // Handle events with timeout (allows periodic redraws)
            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
            {
                self.handle_key_event(key);
            }

            // Drain all pending updates from handler (process all available events)
            while let Ok(tui_event) = rx.try_recv() {
                match tui_event {
                    TuiEvent::Update(screen_vm) => {
                        self.current_screen = Some(*screen_vm);
                        self.error_message = None;
                    }
                    TuiEvent::Error(msg) => {
                        self.error_message = Some(msg);
                        self.notification_message = None;
                    }
                    TuiEvent::Notification(msg) => {
                        self.notification_message = Some(msg);
                    }
                }
            }

            // Exit if quit flag is set
            if self.should_quit {
                // Notify handler that we're quitting
                if let Some(tx) = &self.signal_tx {
                    let _ = tx.send(RendererSignal::Quit);
                }
                break;
            }
        }

        Ok(())
    }

    /// Handle keyboard input (Router pattern)
    ///
    /// Handles global operations first, then delegates to Components.
    fn handle_key_event(&mut self, key: KeyEvent) {
        // Only handle key press events, not release
        if key.kind != KeyEventKind::Press {
            return;
        }

        // 1. Global operations (always prioritized)
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
                return;
            }
            _ => {}
        }

        // 2. Delegate to Dashboard component
        if let Some(screen) = &self.current_screen
            && let Some(_action) = self.dashboard_component.handle_input(key, screen)
        {
            // Future: handle navigation actions here (e.g., show details page)
        }
    }

    /// Render the screen using Components (Router pattern)
    ///
    /// Delegates to Components for rendering.
    fn render(&mut self, f: &mut Frame) {
        let size = f.area();

        // If we have an error, show it
        if let Some(error_msg) = &self.error_message {
            use ratatui::style::{Color, Style};
            use ratatui::text::Span;
            use ratatui::widgets::{Block, Borders, Paragraph};

            let error = Paragraph::new(Span::styled(
                error_msg.as_str(),
                Style::default().fg(Color::Red),
            ))
            .block(Block::default().title("Error").borders(Borders::ALL));

            f.render_widget(error, size);
            return;
        }

        // If we don't have data yet, show loading message
        let Some(screen) = &self.current_screen else {
            use ratatui::widgets::{Block, Borders, Paragraph};

            let loading = Paragraph::new("Waiting for session data...")
                .block(Block::default().title("Loading").borders(Borders::ALL));

            f.render_widget(loading, size);
            return;
        };

        // Delegate to Dashboard component for rendering
        self.dashboard_component.render(f, size, screen);
    }
}

impl Default for TuiRenderer {
    fn default() -> Self {
        Self::new()
    }
}
