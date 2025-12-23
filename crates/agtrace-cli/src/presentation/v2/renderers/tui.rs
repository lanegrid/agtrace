//! TUI Renderer for Watch command (v2)
//!
//! This module implements the TUI event loop and screen rendering.
//! It receives `TuiScreenViewModel` updates via channel and renders them using Ratatui.
//!
//! ## Design:
//! - Renderer owns UI state (scroll positions, selected items)
//! - Renderer does NOT own data (receives ViewModels via channel)
//! - Uses View widgets to render the screen
//! - Handles keyboard input for navigation and control

use std::io;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    Frame, Terminal,
};

use crate::presentation::v2::view_models::TuiScreenViewModel;
use crate::presentation::v2::views::tui::{
    DashboardView, StatusBarView, TimelineView, TurnHistoryView,
};

/// TUI events sent from handler to renderer
pub enum TuiEvent {
    /// Update screen with new ViewModel
    Update(Box<TuiScreenViewModel>),
    /// Fatal error occurred
    Error(String),
}

/// TUI Renderer application state
pub struct TuiRenderer {
    /// Current screen data (received from handler)
    current_screen: Option<TuiScreenViewModel>,

    /// UI State: Timeline scroll offset
    timeline_scroll: u16,

    /// UI State: Should quit flag
    should_quit: bool,

    /// Error message to display (if any)
    error_message: Option<String>,
}

impl TuiRenderer {
    pub fn new() -> Self {
        Self {
            current_screen: None,
            timeline_scroll: 0,
            should_quit: false,
            error_message: None,
        }
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
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key_event(key);
                }
            }

            // Check for updates from handler (non-blocking)
            if let Ok(tui_event) = rx.try_recv() {
                match tui_event {
                    TuiEvent::Update(screen_vm) => {
                        self.current_screen = Some(*screen_vm);
                        self.error_message = None;
                    }
                    TuiEvent::Error(msg) => {
                        self.error_message = Some(msg);
                    }
                }
            }

            // Exit if quit flag is set
            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Handle keyboard input
    fn handle_key_event(&mut self, key: KeyEvent) {
        // Only handle key press events, not release
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            // Quit
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            // Scroll timeline up
            KeyCode::Up | KeyCode::Char('k') => {
                self.timeline_scroll = self.timeline_scroll.saturating_sub(1);
            }
            // Scroll timeline down
            KeyCode::Down | KeyCode::Char('j') => {
                self.timeline_scroll = self.timeline_scroll.saturating_add(1);
            }
            // Page up
            KeyCode::PageUp => {
                self.timeline_scroll = self.timeline_scroll.saturating_sub(10);
            }
            // Page down
            KeyCode::PageDown => {
                self.timeline_scroll = self.timeline_scroll.saturating_add(10);
            }
            // Home (top)
            KeyCode::Home => {
                self.timeline_scroll = 0;
            }
            _ => {}
        }
    }

    /// Render the screen using Views
    fn render(&self, f: &mut Frame) {
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

        // Main layout: [Dashboard | Timeline + Turn History | Status Bar]
        let main_chunks = Layout::vertical([
            Constraint::Length(9), // Dashboard
            Constraint::Min(10),   // Main content area (Timeline + Turns)
            Constraint::Length(3), // Status bar
        ])
        .split(size);

        // Render dashboard
        let dashboard_view = DashboardView::new(&screen.dashboard);
        f.render_widget(dashboard_view, main_chunks[0]);

        // Split main content area into timeline and turn history
        let content_chunks = Layout::horizontal([
            Constraint::Percentage(70), // Timeline
            Constraint::Percentage(30), // Turn history
        ])
        .split(main_chunks[1]);

        // Render timeline
        let timeline_view = TimelineView::new(&screen.timeline);
        f.render_widget(timeline_view, content_chunks[0]);

        // Render turn history
        let turn_history_view = TurnHistoryView::new(&screen.turn_history);
        f.render_widget(turn_history_view, content_chunks[1]);

        // Render status bar
        let status_bar_view = StatusBarView::new(&screen.status_bar);
        f.render_widget(status_bar_view, main_chunks[2]);
    }
}

impl Default for TuiRenderer {
    fn default() -> Self {
        Self::new()
    }
}
