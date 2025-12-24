# Session ID Handling - Robust Design

## Problem Statement

Current session_id handling is fragile:

1. **No type distinction** between short IDs, full IDs, user input, and resolved IDs
2. **Resolution logic scattered** across multiple layers (SessionService, WatchService)
3. **Ambiguous semantics** - when to resolve? when to fail? when to fallback?
4. **Bug-prone** - recent bug: `SessionUpdated` with unindexed full ID failed resolution

## Design Goals

1. **Type safety** - prevent mixing short/full/unresolved IDs
2. **Explicit resolution** - clear boundaries where resolution happens
3. **Fail-safe** - predictable behavior for edge cases (new sessions, missing index)
4. **Centralized logic** - single source of truth for resolution

## Design Options

### Option A: Type-Based Approach

```rust
// Domain types
struct SessionId(String);           // Full UUID (validated)
struct ShortSessionId(String);      // 8-char prefix (validated)
enum SessionIdInput {               // User input (unvalidated)
    Short(String),
    Full(String),
}

// Resolution service
impl SessionResolver {
    fn resolve(&self, input: SessionIdInput) -> Result<SessionId> {
        // DB lookup → prefix search → error
    }

    fn resolve_or_passthrough(&self, input: SessionIdInput) -> SessionId {
        // DB lookup → prefix search → use as-is (for filesystem fallback)
    }
}
```

**Pros:**
- Type safety prevents mixing contexts
- Clear resolution boundaries
- Explicit fallback strategy

**Cons:**
- More types to manage
- Conversion overhead
- API changes required

### Option B: Service-Based Approach (Lighter)

```rust
// Keep String/&str, but centralize resolution

struct SessionResolver {
    db: Arc<Mutex<Database>>,
}

impl SessionResolver {
    // Strict resolution (for user input)
    fn resolve_strict(&self, id_or_prefix: &str) -> Result<String> {
        self.resolve_from_db(id_or_prefix)
            .ok_or_else(|| anyhow!("Session not found: {}", id_or_prefix))
    }

    // Best-effort resolution (for system events)
    fn resolve_best_effort(&self, id_or_prefix: &str) -> String {
        self.resolve_from_db(id_or_prefix)
            .unwrap_or_else(|| id_or_prefix.to_string())
    }

    fn resolve_from_db(&self, id: &str) -> Option<String> {
        let db = self.db.lock().unwrap();
        db.get_session_by_id(id)
            .ok().flatten()
            .map(|s| s.id)
            .or_else(|| db.find_session_by_prefix(id).ok().flatten())
    }
}
```

**Pros:**
- Minimal API changes
- Centralized logic
- Clear semantics (strict vs best-effort)

**Cons:**
- No compile-time guarantees
- Still relies on conventions

### Option C: Hybrid Approach

Combine service-based resolution with lightweight type markers:

```rust
// Marker types (zero-cost abstractions)
struct Resolved<T>(T);              // Type-level marker: "this ID was resolved"
struct Unresolved<T>(T);            // Type-level marker: "this ID needs resolution"

// Resolution service
impl SessionResolver {
    fn resolve(&self, id: Unresolved<&str>) -> Result<Resolved<String>> {
        // Strict resolution for user input
    }

    fn resolve_or_use(&self, id: Unresolved<&str>) -> Resolved<String> {
        // Best-effort for system events
    }
}

// APIs require Resolved<String>
impl WatchService {
    fn watch_session(&self, id: Resolved<String>) -> Result<StreamHandle> {
        // No resolution needed here - caller handles it
    }
}
```

**Pros:**
- Type safety without proliferation
- Zero runtime cost
- Clear contracts

**Cons:**
- Phantom types can confuse
- Marker wrapping/unwrapping

## Recommendation

**Start with Option B (Service-Based)**, then evolve to Option C if needed.

### Rationale

1. **Immediate fix** - solves the current bug with minimal disruption
2. **Clear semantics** - `strict` vs `best_effort` is self-documenting
3. **Centralized** - all resolution logic in one place
4. **Future-proof** - can add types later without breaking changes

### Implementation Plan

1. Create `SessionResolver` in `agtrace-runtime`
2. Replace scattered resolution with `resolver.resolve_strict()` (user input) and `resolver.resolve_best_effort()` (system events)
3. Update `watch_session` to use `resolve_best_effort` (handles new sessions)
4. Update CLI handlers to use `resolve_strict` (user must provide valid ID)

### Edge Cases Handled

| Scenario | Input | Strategy | Result |
|----------|-------|----------|--------|
| User types short ID | `528cd366` | strict | DB lookup → full ID or error |
| User types full ID | `528cd366-...` | strict | DB lookup → full ID or error |
| System event (indexed) | `528cd366-...` | best-effort | DB lookup → full ID |
| System event (new session) | `f51c245d-...` | best-effort | DB miss → use as-is → filesystem scan |
| Invalid short ID | `99999999` | strict | Error |

## Next Steps

1. Implement `SessionResolver` service
2. Update `WatchService::watch_session` to use best-effort resolution
3. Update CLI handlers to use strict resolution
4. Add unit tests for edge cases
5. Document resolution semantics in code
