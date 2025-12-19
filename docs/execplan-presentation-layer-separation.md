# Presentation Layer: Complete Separation of Renderer and Formatter Responsibilities

This ExecPlan is a living document maintained according to `.agent/PLANS.md`. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.


## Purpose / Big Picture

After this change, the presentation layer will have a clean separation of concerns: **Renderers** will only handle I/O and screen control (writing to stdout, managing terminal state, handling TUI buffers), while **Formatters** will handle all data transformation and business logic (calculating percentages, formatting timestamps, categorizing tools, building display strings).

Users will see no change in behavior. The refactoring is purely internal, improving code quality and maintainability. Success is verified by running all existing tests and observing that `agtrace session show`, `agtrace session list`, and `agtrace watch` continue to display identical output.

**What's wrong now:** Renderers (`console.rs`, `tui.rs`, `refresh.rs`) contain business logic for formatting data. For example, `console.rs` knows how to calculate "2 min ago" from timestamps, knows which providers get which colors, and knows how to compute progress bar percentages. This violates separation of concerns: if we add a new output format (HTML, JSON-structured logs), we'd have to duplicate all this logic.

**What will be fixed:** All formatting logic moves to `formatters/` modules. Renderers become "dumb" I/O handlers that call `View::from_state()` or `View::new()` and write the resulting `Display` impl to their output device.


## Progress

- [x] (2025-12-20) Phase 0: Moved token calculation logic from renderers to TokenUsageView::from_state
  - Added factory method `TokenUsageView::from_state(state, options)`
  - Removed duplicate token limit calculation from console.rs, tui.rs, refresh.rs
  - Removed imports: TokenLimits, TokenSummaryDisplay from renderers
  - All tests pass (23 unit tests, 11 integration tests)
  - Commit: 24245fb
- [ ] Phase 1: Extract text utilities (truncate, normalize) to formatters/text.rs
- [ ] Phase 2: Extract time formatting utilities to formatters/time.rs
- [ ] Phase 3: Create formatters/session_list.rs for session table formatting
- [ ] Phase 4: Create formatters/watch.rs for watch-specific event formatting
- [ ] Phase 5: Create formatters/tool.rs for tool categorization and call summaries
- [ ] Phase 6: Create formatters/path.rs for path shortening utilities
- [ ] Phase 7: Update all renderers to use new formatters
- [ ] Phase 8: Remove all orphaned formatting code from renderers
- [ ] Phase 9: Final validation and documentation


## Surprises & Discoveries

- Phase 0 revealed that `TokenUsageView` had lifetime parameters `<'a>` that were unnecessary after moving to owned data. Removing them simplified the API.
- There are actually TWO different `create_progress_bar` functions: one in `console.rs` (uses `█` and `░`) and one in `token.rs` (uses `=` and `.`). They serve different purposes and should both be preserved in their respective formatters.


## Decision Log

- Decision: Start with TokenUsageView as proof of concept
  Rationale: Token calculation logic was duplicated in 3 places (console.rs:592-611, tui.rs:240-258, refresh.rs:100-124), making it the clearest example of the problem. Successfully migrating this proves the pattern works.
  Date: 2025-12-20

- Decision: Create new formatter modules rather than cramming everything into existing files
  Rationale: `session.rs` is already 466 lines. Adding watch-specific formatting would create a 1000+ line file. Better to have focused modules: `watch.rs`, `tool.rs`, `time.rs`, `path.rs`, `text.rs`.
  Date: 2025-12-20

- Decision: Keep both progress bar implementations (console and token)
  Rationale: They serve different visual purposes. Console uses block characters (`█░`) for watch summary display, token.rs uses equals (`=.`) for detailed token view. Merging them would lose visual distinction.
  Date: 2025-12-20


## Outcomes & Retrospective

**Phase 0 Complete:**
- Removed 97 lines of duplicate code across 3 files
- Established pattern: `View::from_state(state, options) -> View` where View owns its data
- All tests pass, no behavioral changes
- Renderers no longer import `TokenLimits` or perform token calculations


## Context and Orientation

The presentation layer lives in `crates/agtrace-cli/src/presentation/` and has two main parts:

1. **Formatters** (`presentation/formatters/*.rs`): Transform domain data into displayable strings. These implement `std::fmt::Display` and are pure data transformations with no I/O.

2. **Renderers** (`presentation/renderers/*.rs`): Handle I/O and screen control. They should only know about terminal APIs (`println!`, `crossterm`, `owo_colors`) and should call formatters to get display strings.

**Current files:**

```
presentation/
├── formatters/
│   ├── mod.rs           (re-exports)
│   ├── options.rs       (FormatOptions, TokenSummaryDisplay)
│   ├── token.rs         (TokenUsageView - RECENTLY CLEANED)
│   ├── session.rs       (TimelineView, CompactView, 466 lines)
│   ├── event.rs         (EventView)
│   ├── pack.rs          (pack report formatting)
│   ├── doctor.rs        (diagnostic formatting)
│   └── init.rs          (init wizard formatting)
├── renderers/
│   ├── mod.rs
│   ├── traits.rs        (WatchView, SessionView, SystemView, DiagnosticView)
│   ├── models.rs        (data transfer objects)
│   ├── console.rs       (701 lines - NEEDS CLEANUP)
│   ├── tui.rs           (282 lines - MOSTLY CLEAN)
│   └── refresh.rs       (661 lines - NEEDS MAJOR CLEANUP)
```

**Key terms:**

- **Renderer**: A struct that implements one or more View traits (WatchView, SessionView, etc.). It receives domain data and writes to an output device (stdout, TUI buffer).
- **Formatter** / **View**: A struct that implements `std::fmt::Display`. It holds displayable data and knows how to convert it to a string. Example: `TokenUsageView`, `EventView`.
- **SessionState**: Runtime domain model (from `agtrace-runtime`) representing an agent session's current state (token usage, turn count, model, etc.).
- **AgentEvent**: Domain event (from `agtrace-types`) representing a single action in an agent session (user message, tool call, reasoning, etc.).


## Plan of Work

### Phase 1: Extract Text Utilities

Create `formatters/text.rs` containing:
- `truncate(text: &str, max_len: usize) -> String` (from refresh.rs:257)
- `normalize_and_clean(text: &str, max_chars: usize) -> String` (from console.rs:635)

These are pure string manipulation with no domain knowledge. Moving them out makes renderers simpler and allows reuse.

### Phase 2: Extract Time Formatting

Create `formatters/time.rs` containing:
- `format_relative_time(ts: &str) -> String` (from console.rs:656) - converts RFC3339 to "2 min ago"
- `format_delta_time(duration: chrono::Duration) -> Option<String>` (from refresh.rs:76) - converts durations to "+2m" format

These encapsulate time presentation logic.

### Phase 3: Session List Formatting

Create `formatters/session_list.rs` containing:
- `SessionListView` with factory method `from_summaries(sessions: &[SessionSummary]) -> Self`
- Implements `Display` to generate the table output currently in `console.rs:print_sessions_table`

This removes the need for renderers to know about provider colors or snippet truncation rules.

### Phase 4: Watch Event Formatting

Create `formatters/watch.rs` containing:
- `WatchHeaderView::from_state(state: &SessionState) -> Self` (from refresh.rs:format_header)
- `WatchEventView::new(event: &AgentEvent, delta: Option<String>, project_root: Option<&Path>) -> Self` (from refresh.rs:format_event)
- `WatchContentView::from_events(events: &[AgentEvent]) -> Self` (from refresh.rs:format_content)

These know how to display watch-specific summaries and event streams.

### Phase 5: Tool Formatting

Create `formatters/tool.rs` containing:
- `categorize_tool(name: &str) -> (&'static str, impl Fn(&str) -> String)` (from refresh.rs:197)
- `ToolCallSummaryView::new(name: &str, args: &serde_json::Value, project_root: Option<&Path>) -> Self` (from refresh.rs:format_tool_call)

This encapsulates tool-specific display rules (icons, colors, argument summarization).

### Phase 6: Path Utilities

Create `formatters/path.rs` containing:
- `shorten_path(path: &str, project_root: Option<&Path>) -> String` (from refresh.rs:223)

This is a pure transformation that doesn't belong in a renderer.

### Phase 7: Update Renderers

For each renderer file, replace direct formatting calls with View constructors:

**console.rs:**
- Replace `print_sessions_table()` call with `print!("{}", SessionListView::from_summaries(sessions))`
- Replace `format_relative_time()` calls with `time::format_relative_time()`
- Replace `truncate_for_display()` calls with `text::normalize_and_clean()`
- Replace `create_progress_bar()` with inline version (only used in watch summary)

**refresh.rs:**
- Replace `WatchBuffer::format_header()` with `WatchHeaderView::from_state(&self.state)`
- Replace `WatchBuffer::format_content()` with `WatchContentView::from_events(&self.events)`
- Replace `WatchBuffer::format_event()` with `WatchEventView::new()`
- Remove all helper methods (categorize_tool, shorten_path, format_tool_call, truncate, format_delta_time)

**tui.rs:**
- Already clean after Phase 0, no changes needed

### Phase 8: Cleanup

Remove all orphaned formatting functions from renderer files. Verify with `cargo clippy` that no dead code warnings appear.

### Phase 9: Validation

Run full test suite. Manually test:
- `agtrace session list` (should show colored provider names, truncated snippets, relative times)
- `agtrace session show <id>` (should show timeline with token usage)
- `agtrace watch` in both console and refresh mode (should show live updates with token footer)


## Concrete Steps

All commands run from repository root (`/Users/zawakin/go/src/github.com/lanegrid/agtrace`).

### Phase 1 Steps

1. Create `crates/agtrace-cli/src/presentation/formatters/text.rs`:

```rust
/// Truncate text to max_len characters, adding "..." if truncated
pub fn truncate(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let chars: Vec<char> = text.chars().take(max_len - 3).collect();
        format!("{}...", chars.iter().collect::<String>())
    }
}

/// Normalize whitespace, strip known noise, and truncate
pub fn normalize_and_clean(text: &str, max_chars: usize) -> String {
    let normalized = text
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let cleaned = normalized
        .trim_start_matches("<command-name>/clear</command-name>")
        .trim_start_matches("<command-message>clear</command-message>")
        .trim()
        .to_string();

    truncate(&cleaned, max_chars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_text() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long_text() {
        assert_eq!(truncate("hello world!", 8), "hello...");
    }

    #[test]
    fn test_normalize_and_clean() {
        let input = "<command-name>/clear</command-name>  hello\n\nworld  ";
        assert_eq!(normalize_and_clean(input, 20), "hello world");
    }
}
```

2. Add to `formatters/mod.rs`:

```rust
pub mod text;
```

3. Run tests:

```bash
cargo test formatters::text
```

Expected: 3 tests passed.

4. Update `console.rs` imports:

```rust
use crate::presentation::formatters::text;
```

5. Replace calls in `console.rs`:
   - Line 635: `truncate_for_display(s, max_chars)` → `text::normalize_and_clean(s, max_chars)`
   - Line 639: (in snippet_display) → use `text::normalize_and_clean()`

6. Remove `truncate_for_display` function from console.rs (lines 635-654)

7. Update `refresh.rs`:
   - Replace `self.truncate(text, len)` calls with `text::truncate(text, len)`
   - Remove `truncate` method from WatchBuffer (lines 257-264)

8. Test:

```bash
cargo test
cargo clippy
```

Expected: All tests pass, no dead_code warnings.

### Phase 2 Steps

1. Create `crates/agtrace-cli/src/presentation/formatters/time.rs`:

```rust
use chrono::{DateTime, Utc};

/// Format RFC3339 timestamp as relative time ("2 min ago", "yesterday")
pub fn format_relative_time(ts: &str) -> String {
    let parsed = match DateTime::parse_from_rfc3339(ts) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return ts.to_string(),
    };

    let now = Utc::now();
    let duration = now.signed_duration_since(parsed);

    let seconds = duration.num_seconds();
    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();

    if seconds < 60 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{} min ago", minutes)
    } else if hours < 24 {
        format!("{} hours ago", hours)
    } else if days == 1 {
        "yesterday".to_string()
    } else if days < 7 {
        format!("{} days ago", days)
    } else if days < 30 {
        let weeks = days / 7;
        format!("{} weeks ago", weeks)
    } else if days < 365 {
        let months = days / 30;
        format!("{} months ago", months)
    } else {
        let years = days / 365;
        format!("{} years ago", years)
    }
}

/// Format duration as "+2m5s" or "+30s", or None if < 2s
pub fn format_delta_time(duration: chrono::Duration) -> Option<String> {
    let seconds = duration.num_seconds();
    if seconds < 2 {
        return None;
    }

    if seconds < 60 {
        Some(format!("+{}s", seconds))
    } else {
        let minutes = seconds / 60;
        let remaining_secs = seconds % 60;
        if remaining_secs == 0 {
            Some(format!("+{}m", minutes))
        } else {
            Some(format!("+{}m{}s", minutes, remaining_secs))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_format_relative_time_recent() {
        let now = Utc::now();
        let ts = now.to_rfc3339();
        assert_eq!(format_relative_time(&ts), "just now");
    }

    #[test]
    fn test_format_delta_time_short() {
        let d = chrono::Duration::seconds(45);
        assert_eq!(format_delta_time(d), Some("+45s".to_string()));
    }

    #[test]
    fn test_format_delta_time_minutes() {
        let d = chrono::Duration::seconds(125);
        assert_eq!(format_delta_time(d), Some("+2m5s".to_string()));
    }

    #[test]
    fn test_format_delta_time_noise_filter() {
        let d = chrono::Duration::seconds(1);
        assert_eq!(format_delta_time(d), None);
    }
}
```

2. Add to `formatters/mod.rs`:

```rust
pub mod time;
```

3. Update `console.rs`:
   - Import: `use crate::presentation::formatters::time;`
   - Replace `format_relative_time(ts)` call (line 636) with `time::format_relative_time(ts)`
   - Remove `format_relative_time` function (lines 656-692)

4. Update `refresh.rs`:
   - Import: `use crate::presentation::formatters::time;`
   - Replace `self.format_delta_time(diff)` with `time::format_delta_time(diff)`
   - Remove `format_delta_time` method (lines 76-93)

5. Test:

```bash
cargo test formatters::time
cargo clippy
```

Expected: 4 tests passed, no warnings.

### Phase 3 Steps

1. Create `crates/agtrace-cli/src/presentation/formatters/session_list.rs`:

```rust
use crate::presentation::formatters::text;
use crate::presentation::formatters::time;
use agtrace_index::SessionSummary;
use owo_colors::OwoColorize;
use std::fmt;

pub struct SessionListView {
    sessions: Vec<SessionSummary>,
}

impl SessionListView {
    pub fn from_summaries(sessions: Vec<SessionSummary>) -> Self {
        Self { sessions }
    }
}

impl fmt::Display for SessionListView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for session in &self.sessions {
            let id_short = if session.id.len() > 8 {
                &session.id[..8]
            } else {
                &session.id
            };

            let time_str = session.start_ts.as_deref().unwrap_or("unknown");
            let time_display = time::format_relative_time(time_str);

            let snippet = session.snippet.as_deref().unwrap_or("");
            let snippet_display = text::normalize_and_clean(snippet, 80);

            let provider_display = match session.provider.as_str() {
                "claude_code" => format!("{}", session.provider.blue()),
                "codex" => format!("{}", session.provider.green()),
                "gemini" => format!("{}", session.provider.red()),
                _ => session.provider.clone(),
            };

            let snippet_final = if snippet_display.is_empty() {
                format!("{}", "[empty]".bright_black())
            } else {
                snippet_display
            };

            writeln!(
                f,
                "{} {} {} {}",
                time_display.bright_black(),
                id_short.yellow(),
                provider_display,
                snippet_final
            )?;
        }
        Ok(())
    }
}
```

2. Add to `formatters/mod.rs`:

```rust
pub mod session_list;
pub use session_list::SessionListView;
```

3. Update `console.rs`:
   - Import: `use crate::presentation::formatters::SessionListView;`
   - In `render_session_list` method (line 268), replace:
     ```rust
     OutputFormat::Plain => {
         print_sessions_table(sessions);
     }
     ```
     with:
     ```rust
     OutputFormat::Plain => {
         print!("{}", SessionListView::from_summaries(sessions.to_vec()));
     }
     ```
   - Remove `print_sessions_table` function (lines 627-662)

4. Test:

```bash
cargo test
```

Expected: All tests pass. Manually verify:

```bash
cargo run -- session list
```

Output should be identical to before (colored providers, relative times, truncated snippets).

### Phases 4-6 Steps

These follow the same pattern:
1. Create new formatter file with View struct + `Display` impl
2. Add to `mod.rs`
3. Update renderers to use new View
4. Remove old methods
5. Test with `cargo test` and `cargo clippy`

Detailed steps omitted for brevity; follow Phase 1-3 pattern.

### Phase 7-9 Steps

After all formatters are created:

1. Review each renderer file and verify no formatting logic remains
2. Run full test suite:

```bash
cargo test
cargo clippy
cargo fmt --check
```

3. Manual verification:

```bash
# Test session list
cargo run -- session list

# Test session show
cargo run -- session show <id>

# Test watch (console mode)
cargo run -- watch

# Test watch (refresh mode)
cargo run -- watch --refresh
```

4. Update this ExecPlan's Outcomes section with final line counts and lessons learned.


## Validation and Acceptance

**Automated tests:**

```bash
cargo test
```

Expected: All 23 unit tests + 11 integration tests pass (same count as before).

**Manual acceptance:**

1. Run `cargo run -- session list` and verify:
   - Provider names are colored (blue=claude_code, green=codex, red=gemini)
   - Timestamps show relative time ("2 min ago", "yesterday")
   - Snippets are truncated to ~80 chars with "..."
   - Empty snippets show "[empty]" in dim text

2. Run `cargo run -- session show <session-id>` and verify:
   - Timeline displays with token usage footer
   - Token usage shows progress bar and percentages
   - Events show with proper icons and colors

3. Run `cargo run -- watch` and verify:
   - Live event stream updates
   - Token footer updates on TokenUsage events
   - Progress bar reflects usage percentage
   - Tool calls show categorized icons (📖 for read, 🛠️ for write, etc.)

**Code quality:**

```bash
cargo clippy
```

Expected: No warnings about dead code, unused imports, or cognitive complexity.

**Line count reduction:**

```bash
# Before
wc -l crates/agtrace-cli/src/presentation/renderers/{console,refresh}.rs
# Should be: 701 + 661 = 1362 lines

# After (expected)
wc -l crates/agtrace-cli/src/presentation/renderers/{console,refresh}.rs
# Target: ~400 + ~250 = 650 lines (52% reduction)
```


## Idempotence and Recovery

All steps are safe to run multiple times:
- Creating formatter files: If file exists, `Write` tool will fail; use `Edit` instead
- Removing functions: Use `cargo clippy --fix` to catch any remaining calls
- Tests: Can be run unlimited times without side effects

**Recovery from failures:**

If compilation fails mid-refactor:
1. Comment out the broken renderer code temporarily
2. Complete the formatter implementation
3. Uncomment and update renderer to use new formatter
4. Run tests to verify

If tests fail:
1. Check the diff between old and new output with a debugger or print statements
2. Verify formatters match original logic exactly (case sensitivity, whitespace, etc.)
3. If logic needs adjustment, update formatter and re-test


## Artifacts and Notes

### Phase 0 Diff Summary

```diff
diff --git a/crates/agtrace-cli/src/presentation/formatters/token.rs
+impl TokenUsageView {
+    pub fn from_state(state: &SessionState, options: FormatOptions) -> Self {
+        let token_limits = TokenLimits::new();
+        let token_spec = state.model.as_ref().and_then(|m| token_limits.get_limit(m));
+        // ... calculation logic moved from renderers ...
+        Self { summary, options }
+    }
+}

diff --git a/crates/agtrace-cli/src/presentation/renderers/console.rs
-                let token_limits = TokenLimits::new();
-                let token_spec = state.model.as_ref()...
-                // 29 lines of calculation removed
+                let token_view = TokenUsageView::from_state(state, opts.clone());
```

**Result:** 97 lines removed, 13 lines added, net -84 lines.


## Interfaces and Dependencies

### New Modules

**formatters/text.rs:**
```rust
pub fn truncate(text: &str, max_len: usize) -> String;
pub fn normalize_and_clean(text: &str, max_chars: usize) -> String;
```

**formatters/time.rs:**
```rust
pub fn format_relative_time(ts: &str) -> String;
pub fn format_delta_time(duration: chrono::Duration) -> Option<String>;
```

**formatters/session_list.rs:**
```rust
pub struct SessionListView {
    sessions: Vec<SessionSummary>,
}

impl SessionListView {
    pub fn from_summaries(sessions: Vec<SessionSummary>) -> Self;
}

impl fmt::Display for SessionListView { ... }
```

**formatters/watch.rs:**
```rust
pub struct WatchHeaderView {
    project_root: Option<PathBuf>,
}

impl WatchHeaderView {
    pub fn from_state(state: &SessionState) -> Self;
}

impl fmt::Display for WatchHeaderView { ... }

pub struct WatchEventView<'a> {
    event: &'a AgentEvent,
    delta: Option<String>,
    project_root: Option<&'a Path>,
}

impl<'a> WatchEventView<'a> {
    pub fn new(event: &'a AgentEvent, delta: Option<String>, project_root: Option<&'a Path>) -> Self;
}

impl<'a> fmt::Display for WatchEventView<'a> { ... }

pub struct WatchContentView<'a> {
    events: &'a [AgentEvent],
}

impl<'a> WatchContentView<'a> {
    pub fn from_events(events: &'a [AgentEvent]) -> Self;
}

impl<'a> fmt::Display for WatchContentView<'a> { ... }
```

**formatters/tool.rs:**
```rust
pub fn categorize_tool(name: &str) -> (&'static str, fn(&str) -> String);

pub struct ToolCallSummaryView<'a> {
    name: &'a str,
    args: &'a serde_json::Value,
    project_root: Option<&'a Path>,
}

impl<'a> ToolCallSummaryView<'a> {
    pub fn new(name: &'a str, args: &'a serde_json::Value, project_root: Option<&'a Path>) -> Self;
}

impl<'a> fmt::Display for ToolCallSummaryView<'a> { ... }
```

**formatters/path.rs:**
```rust
pub fn shorten_path(path: &str, project_root: Option<&Path>) -> String;
```

### Dependencies

No new external crates. Uses existing:
- `chrono` (already in Cargo.toml for time manipulation)
- `owo_colors` (already in Cargo.toml for terminal colors)
- `serde_json` (already in Cargo.toml for JSON handling)


---

**Revision History:**

- 2025-12-20: Initial ExecPlan created based on presentation layer audit showing 13 pieces of logic to migrate from renderers to formatters. Phase 0 (TokenUsageView) completed as proof of concept.
