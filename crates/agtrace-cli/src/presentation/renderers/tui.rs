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
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};

use crate::presentation::view_models::{
    StatusLevel, TuiScreenViewModel, TurnHistoryViewModel, TurnItemViewModel,
};
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

    /// Debug mode enabled (allows Ctrl+T/Ctrl+D to inject dummy data)
    debug_mode: bool,

    /// Counter for generating unique turn IDs in debug mode
    debug_turn_counter: usize,
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
            debug_mode: false,
            debug_turn_counter: 0,
        }
    }

    pub fn with_signal_sender(mut self, tx: Sender<RendererSignal>) -> Self {
        self.signal_tx = Some(tx);
        self
    }

    /// Enable debug mode (Ctrl+T adds one turn, Ctrl+D adds many turns)
    pub fn with_debug_mode(mut self, enabled: bool) -> Self {
        self.debug_mode = enabled;
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

        // 2. Debug mode operations (Ctrl+T, Ctrl+D)
        if self.debug_mode && key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('t') => {
                    self.debug_add_turns(1);
                    return;
                }
                KeyCode::Char('d') => {
                    self.debug_add_turns(25);
                    return;
                }
                _ => {}
            }
        }

        // 3. Delegate to Dashboard component
        if let Some(screen) = &self.current_screen
            && let Some(_action) = self.dashboard_component.handle_input(key, screen)
        {
            // Future: handle navigation actions here (e.g., show details page)
        }
    }

    /// Debug: Add dummy turns to test scrolling
    fn debug_add_turns(&mut self, count: usize) {
        // Ensure we have a screen to modify
        if self.current_screen.is_none() {
            self.current_screen = Some(Self::create_debug_screen());
        }

        // Generate turns first (before borrowing screen)
        let mut new_turns = Vec::with_capacity(count);
        for _ in 0..count {
            self.debug_turn_counter += 1;
            let turn = Self::create_debug_turn(self.debug_turn_counter);
            new_turns.push(turn);
        }

        let screen = self.current_screen.as_mut().unwrap();

        // Clear waiting state if present
        screen.turn_history.waiting_state = None;

        // Add all new turns
        screen.turn_history.turns.extend(new_turns);

        // Update active turn index to the last turn
        let total_turns = screen.turn_history.turns.len();
        if total_turns > 0 {
            screen.turn_history.active_turn_index = Some(total_turns - 1);

            // Mark only the last turn as active
            for (i, turn) in screen.turn_history.turns.iter_mut().enumerate() {
                turn.is_active = i == total_turns - 1;
            }
        }

        // Update status bar
        screen.status_bar.turn_count = total_turns;
        screen.status_bar.status_message = format!("[DEBUG] {} turns", total_turns);
    }

    /// Create a dummy turn for debugging
    fn create_debug_turn(turn_id: usize) -> TurnItemViewModel {
        use crate::presentation::view_models::StepPreviewViewModel;

        // Simulate progressive context usage
        let max_context = 200_000u32;
        let tokens_per_turn = 5_000u32;
        let prev_total = (turn_id.saturating_sub(1) as u32) * tokens_per_turn;
        let delta_tokens = tokens_per_turn;
        let total = prev_total + delta_tokens;

        let usage_ratio = (total as f64 / max_context as f64).min(1.0);
        let prev_ratio = (prev_total as f64 / max_context as f64).min(1.0);
        let delta_ratio = usage_ratio - prev_ratio;

        // Calculate bar widths (max 20 chars)
        let bar_width = (usage_ratio * 20.0).round() as u16;
        let prev_bar_width = (prev_ratio * 20.0).round() as u16;

        // Color based on usage
        let delta_color = if usage_ratio > 0.8 {
            StatusLevel::Error
        } else if usage_ratio > 0.6 {
            StatusLevel::Warning
        } else {
            StatusLevel::Info
        };

        let messages = [
            "Help me fix this bug",
            "Add a new feature",
            "Refactor this code",
            "Write tests for this",
            "Explain how this works",
            "Update the documentation",
            "Debug this issue",
            "Optimize performance",
        ];

        TurnItemViewModel {
            turn_id,
            title: messages[turn_id % messages.len()].to_string(),
            slash_command: None,
            is_active: false, // Will be set by caller
            is_heavy: delta_ratio > 0.1,
            context_compacted: false,
            prev_total,
            delta_tokens,
            usage_ratio,
            prev_ratio,
            delta_ratio,
            bar_width,
            prev_bar_width,
            delta_color,
            recent_steps: vec![StepPreviewViewModel {
                timestamp: chrono::Utc::now(),
                icon: "🔧".to_string(),
                description: format!("Processing turn {}", turn_id),
                token_usage: Some(delta_tokens),
            }],
            start_time: Some(chrono::Utc::now()),
            child_streams: vec![],
        }
    }

    /// Create a minimal debug screen
    fn create_debug_screen() -> TuiScreenViewModel {
        use crate::presentation::view_models::{
            ContextBreakdownViewModel, DashboardViewModel, StatusBarViewModel, TimelineViewModel,
        };

        TuiScreenViewModel {
            dashboard: DashboardViewModel {
                title: "[DEBUG MODE]".to_string(),
                sub_title: Some("Ctrl+T: add 1 turn, Ctrl+D: add 25 turns".to_string()),
                session_id: "debug-session".to_string(),
                project_root: Some("/debug/project".to_string()),
                log_path: None,
                model: Some("debug-model".to_string()),
                start_time: chrono::Utc::now(),
                last_activity: chrono::Utc::now(),
                elapsed_seconds: 0,
                context_total: 0,
                context_limit: Some(200_000),
                context_usage_pct: Some(0.0),
                context_color: StatusLevel::Info,
                context_breakdown: ContextBreakdownViewModel {
                    fresh_input: 0,
                    cache_creation: 0,
                    cache_read: 0,
                    output: 0,
                    total: 0,
                },
            },
            timeline: TimelineViewModel {
                events: vec![],
                total_count: 0,
                displayed_count: 0,
            },
            turn_history: TurnHistoryViewModel {
                turns: vec![],
                active_turn_index: None,
                waiting_state: None,
            },
            status_bar: StatusBarViewModel {
                event_count: 0,
                turn_count: 0,
                status_message: "[DEBUG] Press Ctrl+T or Ctrl+D".to_string(),
                status_level: StatusLevel::Info,
            },
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
