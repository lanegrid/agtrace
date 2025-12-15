# Progress: `agtrace watch` MVP Implementation (v0.1.0)

## Goal
Implement a minimal viable `watch` command that provides a "formatted stream" experience - much better than `tail -f`, without the complexity of a full TUI.

## Design Principles
1. **Auto-attach**: Automatically watch the latest log file and switch to new sessions when created
2. **Smart formatting**: Convert raw JSON events to icon-colored one-line summaries
3. **Safety alerts**: Highlight errors and potentially dangerous operations

## Status: ✅ Core Implementation Complete

### Phase 1: Setup & Dependencies
- [x] Add `notify` crate dependency to agtrace-cli/Cargo.toml

### Phase 2: Core Implementation
- [x] Create `src/handlers/watch.rs` with basic structure
- [x] Implement file watching logic and event loop
  - Recursive directory watching for provider subdirectories
  - Auto-detection of latest log file
  - Real-time file change monitoring
- [x] Implement smart formatting with icons and colors
  - Event categorization (Info/Edit/Test/Thinking/User/Error)
  - Icon and color mapping (📖/🛠️/🧪/🧠/👤/❌)
  - Text truncation for long content
  - Safety alerts for path traversal and root access

### Phase 3: Integration
- [x] Add `Watch` command to `args.rs`
- [x] Wire up watch handler in `commands.rs`
- [x] Update handlers/mod.rs to export watch module

### Phase 4: Testing & Polish
- [x] Run cargo build and fix compilation errors
- [x] Manual testing - verified auto-attach to latest session
- [x] Run cargo fmt and cargo clippy - all warnings fixed
- [x] Commit the implementation

## Implementation Notes
- Using stdout append-only (no TUI) for simplicity and pipe-ability
- File offset tracking to read only new content
- `notify` crate for file system events with recursive monitoring
- Icon/color scheme defined in design spec
- Provider-agnostic design with v2 event schema

## Known Limitations (Future Work)
- Currently expects v2 schema events (JSONL)
- Provider-specific raw formats require conversion
- Consider adding --follow-current flag to stay on one session
