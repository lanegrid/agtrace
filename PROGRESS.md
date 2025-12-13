# Schema Enhancement: Context and Policy Fields

## Goal
Add `context` and `policy` fields to AgentEventV1 to capture execution environment and agent control policies, preventing data loss currently occurring in `raw` field.

## Implementation Plan

### Phase 1: Type Definitions (agtrace-types)
- [x] Add `GitContext` struct with branch, commit, is_dirty fields
- [x] Add `RunContext` struct with cwd, git, runtime fields
- [x] Add `AgentPolicy` struct with sandbox_mode, network_access, approval_policy fields
- [x] Update `AgentEventV1` to include optional context and policy fields
- [x] Update `AgentEventV1::new()` to initialize new fields as None

### Phase 2: Codex Provider Mapping (agtrace-providers/codex)
- [x] Extract context from `SessionMetaPayload`:
  - cwd → context.cwd
  - cli_version → context.runtime
  - git.branch → context.git.branch
  - git.commit_hash → context.git.commit
- [x] Extract policy from `TurnContextPayload`:
  - approval_policy → policy.approval_policy
  - sandbox_policy → policy.sandbox_mode (handle both Simple and Detailed variants)
  - sandbox_policy.network_access → policy.network_access

### Phase 3: Claude Provider Mapping (agtrace-providers/claude)
- [x] Extract context from `UserRecord`/`AssistantRecord`:
  - cwd → context.cwd
  - git_branch → context.git.branch
- [x] Populate context.runtime with provider name + version if available

### Phase 4: Testing
- [x] Run existing tests to ensure backward compatibility
- [x] Verify snapshot tests still pass
- [x] Run cargo fmt and cargo clippy
- [x] Test with real data from each provider

### Phase 5: Commit
- [ ] Commit with message: "feat: add context and policy fields to capture environment and agent constraints"

## Data Mapping Reference

### Codex
```
SessionMetaPayload → context:
  - cwd → context.cwd
  - cli_version → context.runtime
  - git.branch → context.git.branch
  - git.commit_hash → context.git.commit

TurnContextPayload → policy:
  - approval_policy → policy.approval_policy
  - sandbox_policy.type or .mode → policy.sandbox_mode
  - sandbox_policy.network_access → policy.network_access
```

### Claude
```
UserRecord/AssistantRecord → context:
  - cwd → context.cwd
  - git_branch → context.git.branch
  - version → context.runtime (combined with "claude_code")
```

## Notes
- Fields use `#[serde(skip_serializing_if = "Option::is_none")]` to avoid bloating JSON
- Context typically set on Meta/Start events
- Policy set when turn context changes
- Backward compatible - existing fields unchanged
