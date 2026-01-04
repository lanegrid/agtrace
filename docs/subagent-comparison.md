# Subagent Structure Comparison: Claude Code vs Codex

## Executive Summary

Both Claude Code and Codex support subagents, but with fundamentally different architectural approaches:

- **Claude Code**: Subagents share the parent session's log file
- **Codex**: Subagents create separate session files

## Claude Code Subagent Structure

### File Organization
- Subagent events are recorded **within the same JSONL file** as the parent session
- File path: `~/.claude/projects/<project>/<session-uuid>.jsonl`

### Identification Fields
1. **Tool Call**: `Task` tool with `subagent_type` parameter
   ```json
   {
     "type": "tool_use",
     "name": "Task",
     "input": {
       "subagent_type": "Explore",
       "description": "...",
       "prompt": "..."
     }
   }
   ```

2. **Tool Result**: Contains `agentId` field
   ```json
   {
     "tool_use_id": "toolu_...",
     "type": "tool_result",
     "content": [...],
     "agentId": "ba2ed465"
   }
   ```

### Subagent Types Found
- `Explore`: 23 occurrences
- `general-purpose`: 2 occurrences

### Linking Mechanism
- Parent session UUID: `sessionId` field
- Subagent ID: `agentId` field in tool result
- All events share the same JSONL file

## Codex Subagent Structure

### File Organization
- Subagents create **separate JSONL files**
- File path: `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`
- Each subagent session has its own file

### Identification Fields
1. **Session Metadata**: `source` field in `session_meta` event
   ```json
   {
     "type": "session_meta",
     "payload": {
       "id": "019a6e75-2585-7540-9982-9dced67f1132",
       "source": {"subagent": "review"},
       "originator": "codex_cli_rs",
       "cwd": "/path/to/project"
     }
   }
   ```

2. **Regular Session**: `source` is a string
   ```json
   {
     "source": "cli"
   }
   ```

### Subagent Types Found
- `review`: 2 occurrences

### Linking Mechanism
- No explicit parent reference in the log file
- Temporal relationship (timestamps close together)
- Same `cwd` (current working directory)

### Example Subagent Session
```
File: rollout-2025-11-11T00-49-22-019a6e75-2585-7540-9982-9dced67f1132.jsonl
Size: 14 lines (5.3K)
Content:
- session_meta with source: {"subagent":"review"}
- User message: "Review the current code changes..."
- Automated review workflow
```

## Key Differences

| Aspect | Claude Code | Codex |
|--------|-------------|-------|
| **File Organization** | Same file as parent | Separate file |
| **Identification** | `agentId` in tool_result | `source.subagent` in session_meta |
| **Subagent Types** | Explore, general-purpose | review |
| **Parent Linking** | Implicit (same file) | No explicit link |
| **Session Count** | 1 session, multiple agents | N sessions |

## Implications for agtrace

### Current Implementation Status
The `codex/schema.rs` already supports the `source` field:
```rust
pub source: Value, // Can be string or object like {"subagent":"review"}
```

### Normalization Challenges

1. **Claude Code**:
   - Need to detect `agentId` in tool results
   - Map to agent session within the same file
   - Current implementation likely treats all events as single session

2. **Codex**:
   - Already creates separate sessions
   - Need to link subagent sessions to parent
   - Detect `source.subagent` pattern

### Proposed Enhancements

1. Add `subagent_id` field to `AgentEvent`
2. Add `parent_session_id` field for Codex subagents
3. Update mappers to extract:
   - Claude: `agentId` from tool_result
   - Codex: `subagent` from source field
4. Enhance session listing to show parent-child relationships

## Discovery Commands Used

```bash
# Find Claude Code subagent patterns
grep -h "subagent_type" ~/.claude/projects/*/*.jsonl | \
  grep -o '"subagent_type":"[^"]*"' | sort | uniq -c

# Find Codex subagent patterns
find ~/.codex/sessions -name "*.jsonl" -exec \
  jq -r 'select(.type == "session_meta" and (.payload.source | type) == "object") |
  .payload.source.subagent' {} \; | sort | uniq -c

# Example sessions
# Claude: Any file with Task tool calls
# Codex: rollout-2025-11-11T00-49-22-*.jsonl
```

## References
- Previous investigation session: `a92aef30-bdbf-43f6-a1cb-9ced344500a6`
- Codex schema: `crates/agtrace-providers/src/codex/schema.rs:31`
- Claude sample: `~/.claude/projects/-Users-zawakin-go-src-github-com-lanegrid-agtrace/0126517e-*.jsonl`
