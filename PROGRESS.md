# Progress

## Recent Completions (2025-12-16)

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

## 🚨 Critical Bug: Token Tracking Severely Underreports Usage

### Problem
`agtrace watch` displays **20% usage (40k/200k)** when actual usage is **86% (172k/200k)**.

**4.3x underreporting** - users will hit context limits without warning!

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

## Next Session Actions

1. **Verify schema**: Check `crates/agtrace-types/src/v2.rs` - are cache fields already defined?
2. **Check providers**: Does Claude parser already extract these fields?
3. **If missing**: Add cache token fields to types + parsers
4. **Update tracking**: Modify `watch.rs` accumulation logic
5. **Update display**: Show cache tokens in context window display
6. **Test**: Verify with real session that shows ~172k instead of ~40k

**Critical**: This bug makes the context window display dangerously misleading. Fix before any user-facing release.

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
