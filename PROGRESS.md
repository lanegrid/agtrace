# Token Limits Architecture Implementation

## Goal
Implement resilient, provider-specific context window resolution strategy for `agtrace watch` command.

## Architecture Design
- **Provider Knowledge**: Each provider defines its own model specifications in `models.rs`
- **Resolution Logic**: Centralized resolver in `agtrace-providers/src/token_limits.rs`
- **Longest Prefix Matching**: Handles minor version updates automatically
- **Graceful Degradation**: Show absolute values when limits are unknown

## Implementation Steps

### 1. Provider Model Definitions
Create `models.rs` for each provider:
- `crates/agtrace-providers/src/claude/models.rs`
- `crates/agtrace-providers/src/codex/models.rs`
- `crates/agtrace-providers/src/gemini/models.rs`

### 2. Resolution Logic
- Create `crates/agtrace-providers/src/token_limits.rs`
- Implement longest prefix matching algorithm
- Aggregate definitions from all providers

### 3. CLI Integration
- Update `crates/agtrace-cli/src/token_limits.rs` to use providers implementation
- Maintain backward compatibility with existing API
- Priority: Runtime Metadata > User Config > Provider Knowledge

### 4. Testing & Quality
- Run existing tests
- Update tests to cover new model definitions
- Run cargo fmt and clippy

## Supported Model Specifications (as of 2025-12-17)

### Claude Code
- claude-sonnet-4-5: 200K
- claude-haiku-4-5: 200K
- claude-opus-4-5: 200K
- claude-sonnet-4: 200K
- claude-haiku-4: 200K
- claude-opus-4: 200K
- claude-3-5: 200K
- claude-3: 200K

### Codex/OpenAI
- gpt-5.2: 400K
- gpt-5.1-codex-max: 400K
- gpt-5.1-codex-mini: 400K
- gpt-5.1-codex: 400K
- gpt-5.1: 400K
- gpt-5-codex-mini: 400K
- gpt-5-codex: 400K
- gpt-5: 400K

### Gemini
- gemini-2.5-pro: 1,048,576 (~1M)
- gemini-2.5-flash: 1,048,576 (~1M)
- gemini-2.5-flash-lite: 1,048,576 (~1M)
- gemini-2.0-flash: 1,048,576 (~1M)
- gemini-2.0-flash-lite: 1,048,576 (~1M)

**Legacy models removed**: GPT-4, GPT-4o, Gemini 1.5 series are not included as they were not in the original specification.

## Status
- [x] Create PROGRESS.md
- [x] Implement provider model definitions
  - [x] crates/agtrace-providers/src/claude/models.rs
  - [x] crates/agtrace-providers/src/codex/models.rs
  - [x] crates/agtrace-providers/src/gemini/models.rs
- [x] Implement resolution logic
  - [x] crates/agtrace-providers/src/token_limits.rs
  - [x] Longest prefix matching algorithm
  - [x] Comprehensive test coverage
- [x] Update CLI integration
  - [x] Updated crates/agtrace-cli/src/token_limits.rs
  - [x] Maintained backward compatibility
  - [x] Updated resolution priority documentation
- [x] Run tests and quality checks
  - [x] All 61 CLI tests pass
  - [x] cargo build succeeds
  - [x] cargo fmt clean
  - [x] cargo clippy clean

## Implementation Complete

All tasks completed successfully. The token limits architecture has been refactored according to the design specification:

1. **Provider-specific knowledge**: Each provider now maintains its own model definitions in separate `models.rs` files
2. **Resilient matching**: Longest prefix matching automatically handles new minor versions
3. **Clean separation**: Knowledge layer (providers) is separated from usage layer (CLI)
4. **Backward compatible**: All existing tests pass without modification
5. **Well documented**: Code includes comprehensive comments and test coverage

### Architectural Notes Added

NOTE comments have been added to explain key architectural decisions:

- **token_limits.rs (providers)**: Why distributed definitions, longest prefix matching, and O(n) algorithm
- **token_limits.rs (cli)**: Why thin CLI layer and delegation to providers
- **provider models.rs**: Why struct-based specs instead of tuples or HashMap::insert
- **get_usage_percentage_from_state**: Why runtime metadata takes priority over provider knowledge

These notes document the "why" behind ambiguous design choices, making it easier for future contributors to understand the tradeoffs and maintain the codebase.

### Type Safety Improvements

Model specifications now use dedicated structs instead of tuples:

**Before (tuple-based, position-dependent)**:
```rust
const MODEL_SPECS: &[(&str, u64)] = &[
    ("claude-3-5", 200_000),  // Easy to swap order by mistake
];
```

**After (struct-based, field-named)**:
```rust
const MODEL_SPECS: &[ModelSpec] = &[
    ModelSpec::new("claude-3-5", 200_000),  // Compiler enforces field names
];
```

Benefits:
- **Named fields**: `ModelSpec { prefix, context_window }` self-documents the meaning
- **Compile-time safety**: Cannot swap argument order (`ModelSpec::new(200_000, "claude")` won't compile)
- **Extensibility**: Easy to add fields (e.g., `output_limit`, `cache_support`) without breaking changes
- **Duplicate detection**: Tests verify no duplicate prefixes at compile time
- **IDE support**: Auto-completion works for field names

### Cleanup & Finalization

Legacy model support has been removed to keep the codebase focused and maintainable:

**Removed models** (not in original specification):
- Codex: `gpt-4`, `gpt-4-turbo`, `gpt-4o`, `gpt-4o-mini`
- Gemini: `gemini-1.5-pro`, `gemini-1.5-flash`

**Rationale**:
- Reduces maintenance burden
- Prevents confusion about which models are actively supported
- Keeps implementation aligned with specification
- Tests are cleaner and more focused

All tests pass (61 tests), code is formatted, and clippy shows no warnings.
