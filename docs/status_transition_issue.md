# Status Transition Issue - TUI SATURATION HISTORY

## Problem

The "Status: In Progress" display in SATURATION HISTORY is unstable due to inconsistent logic between real-time streaming and batch session loading.

## Current Implementation

### Two Paths for `is_active` Determination

**Path 1: Real-time Streaming (when `turns_data` is None)**
Location: `crates/agtrace-cli/src/presentation/renderers/tui/mod.rs:200-233`

```rust
// In TokenUsage event handler
if delta > 0 && !app_state.current_user_message.is_empty() {
    let turn_usage = TurnUsageViewModel {
        // ...
        is_active: true,  // ❌ Always true, never updated
        // ...
    };
    app_state.turns_usage.push(turn_usage);
}
```

**Path 2: Batch Loading (when `turns_data` is provided)**
Location: `crates/agtrace-engine/src/session/types.rs:209-240`

```rust
// In AgentSession::compute_turn_metrics()
let is_active = if idx == total_turns.saturating_sub(1) {
    turn.is_active()  // ✅ Based on last step's StepStatus
} else {
    false
};
```

Where `AgentTurn::is_active()` checks:
```rust
pub fn is_active(&self) -> bool {
    self.steps
        .last()
        .map(|step| matches!(step.status, StepStatus::InProgress))
        .unwrap_or(false)
}
```

### When Each Path is Used

- **Real-time path**: When session is not yet assembled (`session = None` in watch handler)
- **Batch path**: When session is assembled and `turns_data` is provided

From `crates/agtrace-cli/src/handlers/watch.rs:263-268`:
```rust
let turns_data = session.as_ref().map(|s| {
    crate::presentation::renderers::tui::build_turns_from_session(
        s,
        max_context,
    )
});
```

## Root Cause

1. **Initial state**: Session not assembled → `turns_data = None`
   - TokenUsage events create turns with `is_active: true`
   - These turns are NEVER updated

2. **After session assembly**: Session assembled → `turns_data = Some(...)`
   - Entire `app_state.turns_usage` is replaced with engine-computed data
   - Engine uses `AgentTurn::is_active()` which checks last step's status
   - Status may differ from the hardcoded `true` in real-time path

3. **Result**: Status flips between "In Progress" and not showing status

## StepStatus Determination Logic

Location: `crates/agtrace-engine/src/session/step_builder.rs:51-81`

```rust
fn determine_status(&self) -> StepStatus {
    // 1. Error check (highest priority)
    if self.tool_executions.iter().any(|t| t.is_error) {
        return StepStatus::Failed;
    }

    // 2. Tool execution status (highest priority for completion)
    if !self.tool_executions.is_empty() {
        // If any tool is missing result, step is in progress
        if self.tool_executions.iter().any(|t| t.result.is_none()) {
            return StepStatus::InProgress;
        }
        // All tools have results -> Done
        return StepStatus::Done;
    }

    // 3. No tools: check message
    if self.message.is_some() {
        return StepStatus::Done;
    }

    // 4. In progress: reasoning only (waiting for next action)
    if self.reasoning.is_some() {
        return StepStatus::InProgress;
    }

    // Default: Done (safe side)
    StepStatus::Done
}
```

## Ideal State Transition

### StepStatus Transitions

```
[Empty]
  ↓
[Reasoning only] → InProgress
  ↓
[Reasoning + ToolCall (no result)] → InProgress
  ↓
[Reasoning + ToolCall + ToolResult (error)] → Failed
  ↓
[Reasoning + ToolCall + ToolResult (success)] → Done
  ↓
[Reasoning + ToolCall + ToolResult + Message] → Done

Alternative path (no tools):
[Reasoning only] → InProgress
  ↓
[Reasoning + Message] → Done
  ↓
[Message only] → Done
```

### TurnStatus (derived from last step)

```
Turn starts (User message arrives)
  ↓
Last step = InProgress → Turn is active
  ↓
Last step = Done/Failed → Turn is inactive
```

## Proposed Solution

### Option 1: Remove Real-time Path (Recommended)

**Remove lines 200-233** in `tui/mod.rs` that create turns in real-time.

**Pros:**
- Single source of truth (engine)
- Consistent logic
- No duplicate code

**Cons:**
- Requires session to be assembled early
- May have delay before turns appear

### Option 2: Unify Real-time Logic with Engine

Move real-time turn creation logic to engine, or call engine methods to determine `is_active`.

**Pros:**
- Can show turns immediately
- Still uses engine logic

**Cons:**
- More complex
- Duplicate computation

### Option 3: Always Provide Assembled Session

Ensure session is always assembled, even in early stage, so `turns_data` is always provided.

**Pros:**
- Simple fix
- No logic changes needed

**Cons:**
- May impact performance
- Requires changes in watch service

## Recommended Approach

**Option 1** is recommended because:
1. It eliminates the inconsistency at the root
2. The batch path is already working correctly
3. Session assembly is fast enough for real-time display
4. Reduces maintenance burden

## Implementation Steps

1. Remove real-time turn creation logic (lines 200-233)
2. Ensure session is assembled frequently enough for responsive UI
3. Test that turns appear correctly during streaming
4. Verify status transitions are stable
