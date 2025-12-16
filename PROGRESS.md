# Progress

## Recent Completions (2025-12-17)

### ✅ Fixed Critical Token Tracking Bug (99% Underreporting)
- Added `cache_creation_input_tokens` tracking to fix severe token underreporting
- Updated v2 schema to include cache creation tokens
- Modified SessionState to track cache_creation_tokens separately
- Updated Claude parser to extract both cache_creation and cache_read tokens
- Enhanced display to show cache token breakdown when usage ≥70%
- **Result**: Accurate token reporting - fixed 99% underreporting (was showing 35k instead of 4.8M tokens)

**Verification**: Session 04abd352 actual total: 4.8M tokens (input: 21k, output: 14k, cache_creation: 185k, cache_read: 4.6M)

**Files**:
- `crates/agtrace-types/src/v2.rs` (added cache_creation_input_tokens)
- `crates/agtrace-cli/src/reactor.rs` (added cache_creation_tokens to SessionState)
- `crates/agtrace-providers/src/v2/claude.rs` (extract cache_creation tokens)
- `crates/agtrace-cli/src/handlers/watch.rs` (accumulate cache_creation tokens)
- `crates/agtrace-cli/src/reactors/tui_renderer.rs` (display cache breakdown)

### ✅ Incremental Session Indexing
- `index update` now skips unchanged files (file size + mtime comparison)
- `session list` auto-refreshes before displaying (disable with `--no-auto-refresh`)
- Database method: `get_all_log_files()` for change detection
- **Result**: Sessions appear in real-time without manual `index update`

**Files**: `crates/agtrace-index/src/db.rs`, `crates/agtrace-cli/src/handlers/{index,session_list}.rs`

### ✅ Claude Code Style Context Window Display
- Visual progress bar with ⛁/⛶ characters
- Progressive detail: compact <70%, expanded ≥70% with input/output/free breakdown
- Warning messages at 80% and 95% thresholds
- Compact token formatting (120k instead of 120000)

**Files**: `crates/agtrace-cli/src/reactors/tui_renderer.rs`

---

## Previous Bug Report (Now Fixed)

### Problem (RESOLVED 2025-12-17)
~~`agtrace watch` displays **20% usage (40k/200k)** when actual usage is **86% (172k/200k)**.~~

~~**4.3x underreporting** - users will hit context limits without warning!~~

**FIXED**: Now correctly tracks all token types including cache_creation and cache_read tokens.

### Root Cause
Claude Code logs contain cache token fields that agtrace ignores:

```json
{
  "input_tokens": 13,
  "cache_creation_input_tokens": 806,      // ❌ NOT TRACKED
  "cache_read_input_tokens": 39405,        // ❌ NOT TRACKED (99% of usage!)
  "output_tokens": 530
}
```

**Current code** (`crates/agtrace-cli/src/handlers/watch.rs:392-393`):
```rust
state.total_input_tokens += usage.input_tokens;   // Only 13
state.total_output_tokens += usage.output_tokens; // Only 530
// Missing: cache_creation + cache_read = 40,211 tokens!
```

### Evidence
Session `f2adf0fa-b11d-41d4-b4ad-edeea394565a`:
- **agtrace shows**: 4,373 tokens (input: 1,484 + output: 2,889)
- **Actual from logs**: 848,106 tokens (including 493,444 cached tokens)
- **Latest turn**: 39,948 tokens (13 new + 806 cache_creation + 39,405 cache_read + 530 output)

### Fix Required

**1. Update Type Schema** (`crates/agtrace-types/src/v2.rs`):
```rust
pub struct TokenUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub cache_creation_input_tokens: Option<i32>,  // ADD
    pub cache_read_input_tokens: Option<i32>,      // ADD
}
```

**2. Update State Tracking** (`crates/agtrace-cli/src/reactor.rs`):
```rust
pub struct SessionState {
    pub total_input_tokens: i32,
    pub total_output_tokens: i32,
    pub total_cache_creation_tokens: i32,  // ADD
    pub total_cache_read_tokens: i32,      // ADD
}
```

**3. Update Accumulation** (`crates/agtrace-cli/src/handlers/watch.rs`):
```rust
EventPayload::TokenUsage(usage) => {
    state.total_input_tokens += usage.input_tokens;
    state.total_output_tokens += usage.output_tokens;
    state.total_cache_creation_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
    state.total_cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
}
```

**4. Update Display** (`crates/agtrace-cli/src/reactors/tui_renderer.rs`):
```rust
let total = input_tokens + output_tokens + cache_creation_tokens + cache_read_tokens;
// Show breakdown:
// ⛁ Input:   13
// ⛁ Output:  530
// ⛁ Cache:   40,211 (creation: 806 + read: 39,405)
```

**5. Update Provider Parsers**:
- Check if Claude Code v2 schema already includes these fields
- Update parsers in `crates/agtrace-providers/src/claude/` if needed

---

## Architecture Notes

### Token Accounting Model
Claude API charges for:
- Fresh input tokens (user messages, system prompts, tools)
- Cache creation tokens (storing context for reuse)
- Cache read tokens (retrieving stored context - cheaper but still counts!)
- Output tokens (model responses)

**All count toward the 200k context window limit**.

### Testing Cache Token Tracking
```bash
# 1. Run watch on active Claude Code session
./target/debug/agtrace watch

# 2. Verify against actual log file
cat ~/.claude/projects/.../session-id.jsonl | \
  jq -s 'map(select(.message.usage)) | last | .message.usage'

# 3. Ensure displayed total matches:
#    input + output + cache_creation + cache_read
```

---

## Previous Completions (Earlier)
- ExecutionContext refactoring
- Reactor architecture (TuiRenderer, TokenUsageMonitor, SafetyGuard, StallDetector)
- Type-safe CLI enums (ProviderName, OutputFormat, OutputStyle, etc.)
- Watch command with event-driven reactors
- Provider abstraction and v2 schema normalization
