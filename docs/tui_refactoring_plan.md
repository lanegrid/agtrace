# TUI Refactoring Plan

## Current Architecture Problems

### 1. Control Flow (Push vs Pull)
**Current (Push型):**
```
WatchService → RuntimeEvent → handler loop → view.render_*() → TUI draws
```
- Handler owns the event loop
- TUI is passive (only renders when called)
- Cannot accept user input (keyboard blocked by handler loop)

**Target (Pull型):**
```
TUI event loop ← {Keyboard, WatchService thread} → TUI draws
```
- TUI owns the event loop
- WatchService runs in background thread
- TUI polls both keyboard input and data events

### 2. Rendering Issues
**Current:**
- Uses raw `crossterm` commands (`queue!`, `MoveTo`, `Clear`)
- Manual cursor position calculations
- Difficult to add complex UI (scroll, split panes, popups)

**Target:**
- Use Ratatui widgets (`List`, `Gauge`, `Block`, `Layout`)
- Declarative UI in `ui()` function
- Easy to extend with new components

### 3. State Management
**Current:**
```rust
struct TuiWatchViewInner {
    events_buffer: VecDeque<String>,  // Just a display cache
    footer_lines: Vec<String>,
    // No scroll position, auto-scroll flag, etc.
}
```

**Target:**
```rust
struct App {
    logs: Vec<String>,
    state: Option<StreamStateViewModel>,
    list_state: ListState,  // Ratatui's scroll state
    auto_scroll: bool,
    scroll_offset: usize,
    should_quit: bool,
}
```

## Refactoring Strategy: Incremental Approach

### Phase 1: Replace crossterm with Ratatui widgets (CURRENT)
**Goal:** Same behavior, better rendering foundation
- Replace `render()` method to use Ratatui's `Terminal<Backend>`
- Use `List` widget for events
- Use `Paragraph` / `Gauge` for footer
- Keep existing Push型 architecture (minimize changes)

**Changes:**
- `tui.rs`: Add `Terminal<CrosstermBackend>` field
- `tui.rs`: Rewrite `render()` using `terminal.draw(|f| ui(f, &inner))`
- `tui.rs`: Create `ui()` function with Ratatui widgets

**Benefits:**
- Foundation for future improvements
- Better rendering (no flicker)
- Easier to extend UI components

### Phase 2: Add event loop with keyboard input
**Goal:** Enable user interactions
- Add event polling in TUI
- Handle keyboard events (q to quit, ↑↓ to scroll)
- Keep WatchService in same thread (for now)

**Changes:**
- Add `TuiEvent` enum (Input, Tick, DataUpdate)
- Add `run()` method with event loop
- Modify `handle_with_view()` to call `run()` for TUI mode

**Benefits:**
- User can quit with 'q'
- Preparation for scroll features

### Phase 3: Move WatchService to background thread
**Goal:** True event-driven architecture
- Run WatchService in separate thread
- Use `mpsc::channel` for communication
- TUI polls keyboard + channel

**Changes:**
- `TuiWatchView`: Add `mpsc::Sender<TuiEvent>`
- `WatchView` trait impl: Send events to channel
- `handle()`: Spawn thread for WatchService
- `run()`: Poll channel in event loop

**Benefits:**
- Non-blocking UI
- Smooth scrolling even during high event rate
- Foundation for advanced features

### Phase 4: Implement full Model-View-Update pattern
**Goal:** Clean separation of concerns
- Extract `App` struct (Model)
- Separate `update()` logic (Update)
- Separate `ui()` rendering (View)

**Changes:**
- Create `App` struct with full state
- Create `App::update(&mut self, event: TuiEvent)`
- Create `ui(f: &mut Frame, app: &App)`

**Benefits:**
- Easier to test (pure functions)
- Easier to add features (state is explicit)
- Better code organization

### Phase 5: Add user interactions
**Goal:** Rich TUI experience
- Scroll up/down with arrow keys
- Auto-scroll toggle with space
- Pause/resume with 'p'
- Filter events with '/'

**Changes:**
- Handle more key events
- Update `App` state based on inputs
- Render scroll indicator, status bar

## Implementation Priority

1. **Phase 1** (Low risk, high value)
   - Minimal code changes
   - Builds foundation
   - Can deploy immediately

2. **Phase 2** (Medium risk, medium value)
   - Adds basic interactivity
   - Tests event loop pattern
   - User-visible improvement

3. **Phase 3** (Medium risk, high value)
   - Architectural shift
   - Unlocks advanced features
   - Requires careful threading

4. **Phase 4-5** (Low risk, high value)
   - Incremental improvements
   - Can be done piece by piece

## Code Structure After Refactoring

```
crates/agtrace-cli/src/presentation/renderers/
├── tui/
│   ├── mod.rs           # Public API
│   ├── app.rs           # App state (Model)
│   ├── event.rs         # TuiEvent enum
│   ├── ui.rs            # Rendering logic (View)
│   └── handler.rs       # Event handling (Update)
└── tui.rs (legacy)      # Or rename to tui_legacy.rs
```

Or simpler (keep in single file for now):
```rust
// tui.rs
enum TuiEvent { ... }
struct App { ... }
impl App { fn update(...) { ... } }
fn ui(f: &mut Frame, app: &App) { ... }
pub struct TuiWatchView { ... }
impl WatchView for TuiWatchView { ... }
```

## Compatibility Considerations

### WatchView Trait
- Must remain compatible for Console rendering
- TUI implementation will send events to channel instead of direct render
- Console implementation unchanged

### Error Handling
- Terminal initialization can fail
- Must restore terminal on panic
- Use Drop trait and panic handlers

### Testing
- Unit tests for App::update()
- Integration tests for event flow
- Manual testing for UI/UX

## Completion Status

### ✅ Phase 1: COMPLETED (2025-12-21)
- Replaced raw `crossterm` with Ratatui widgets
- Implemented `ui()` function using `List` and `Paragraph` widgets
- Added `Terminal<CrosstermBackend>` management
- Same behavior as before, better rendering foundation
- Commit: 4515e73

**Results:**
- ✅ All tests passing
- ✅ No flicker during rendering
- ✅ Foundation for advanced UI features

### ✅ Phase 2+3: COMPLETED (2025-12-21)
- Added `TuiEvent` enum for event handling
- Implemented event loop with keyboard input polling
- Moved WatchService to background thread
- Used `mpsc::channel` for communication
- TUI now owns the event loop (Pull型)
- Keyboard handling: 'q' or 'Esc' to quit
- Commit: bcac1b5

**Results:**
- ✅ All tests passing
- ✅ Non-blocking UI
- ✅ User can quit with 'q'
- ✅ Events processed smoothly from background thread

**Architecture Changes:**
```rust
// Before (Push型)
WatchService → handler loop → view.render_*() → TUI

// After (Pull型)
TUI event loop ← {Keyboard, WatchService thread} → TUI draws
```

### 🔄 Phase 4: OPTIONAL (Future Enhancement)
Extract App state management into cleaner MVU pattern. Current implementation is already functional with state management integrated into the event loop.

**If implemented later:**
- Create dedicated `App` struct
- Separate `update()` and `ui()` functions
- Make code more testable

### 🔄 Phase 5: OPTIONAL (Future Enhancement)
Add advanced user interactions:
- Scroll up/down with arrow keys
- Auto-scroll toggle with space
- Pause/resume with 'p'
- Filter events with '/'
- Status bar with indicators

**Current Status:**
- Basic keyboard handling implemented ('q' to quit)
- Foundation ready for additional keybindings

## Lessons Learned

1. **Incremental Approach Works:** Merging Phase 2+3 was efficient since they were closely related
2. **Privacy Levels Matter:** Used `pub(crate)` for `TuiEvent` to keep it internal while allowing cross-module usage
3. **Borrow Checker:** Cloning data before passing to `terminal.draw()` closure avoided borrow conflicts
4. **Threading:** `mpsc::channel` worked seamlessly for Watch Service communication
5. **Testing:** All existing tests passed without modification, proving backward compatibility

## Next Steps (Optional)

1. Phase 4-5 can be implemented incrementally as needed
2. Current implementation meets all review requirements:
   - ✅ Event-driven architecture (Pull型)
   - ✅ Ratatui widgets usage
   - ✅ User input handling
   - ✅ Background thread for data processing
3. Future enhancements (scroll, filter, etc.) can be added without major refactoring
