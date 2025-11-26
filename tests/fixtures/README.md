# Test Fixtures

This directory contains sample session files for testing agtrace parsers. All data is dummy/synthetic and does not contain real user sessions.

## Codex Format

**Codex uses JSONL format (current, 2025-11+):**

- **Location**: `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`
- **Format**: JSONL (one event per line)
- **Structure**: Each line has `{timestamp, type, payload}`
- **Types**: `session_meta`, `response_item`, `event_msg`, `turn_context`
- **Project info**: Available in `session_meta.payload.cwd` and `turn_context.payload.cwd`
- **Git info**: Available (`git.branch`, `git.commit_hash`, `git.repository_url`)
- **Model**: Available in `turn_context.payload.model` (e.g., "gpt-5-codex")

**Example structure:**
```jsonl
{"timestamp":"2025-11-03T01:46:11.987Z","type":"session_meta","payload":{"id":"...","cwd":"/path/to/project","git":{"branch":"main","commit_hash":"..."}}}
{"timestamp":"2025-11-03T01:46:11.987Z","type":"response_item","payload":{"type":"message","role":"user","content":[...]}}
{"timestamp":"2025-11-03T01:49:22.518Z","type":"turn_context","payload":{"cwd":"/path/to/project","approval_policy":"on-request","model":"gpt-5-codex"}}
```

## Directory Structure

```
tests/fixtures/
├── codex/
│   └── 2025/
│       └── 11/
│           └── 20/
│               ├── rollout-2025-11-20-basic.jsonl        # Basic session
│               └── rollout-2025-11-20-edge-cases.jsonl   # Edge cases
└── claude-code/
    ├── sample-uuid-session.jsonl        # Basic UUID format (main sessions)
    ├── sample-agent-session.jsonl       # Basic Agent format (sidechain sessions)
    ├── edge-cases-session.jsonl         # Edge cases (isMeta, thinkingMetadata, error)
    ├── multi-snapshot-session.jsonl     # Multiple file-history-snapshot
    └── agent-multi-tool-session.jsonl   # Agent with multiple tool calls
```

## File Formats

### 1. Codex Format (`codex/YYYY/MM/DD/*.jsonl`)

**Format**: JSONL (one event per line)

**Key characteristics**:
- Each line is a separate event with `timestamp`, `type`, `payload`
- **Event types**:
  - `session_meta`: Session metadata including **`cwd`** and **`git` info**
  - `response_item`: User/assistant messages
  - `event_msg`: Event messages
  - `turn_context`: Context including **`cwd`** and approval policy
- **Project path**: Available in `session_meta.payload.cwd` and `turn_context.payload.cwd`
- **Git info**: Available in `session_meta.payload.git.branch` and `commit_hash`
- **Model**: `turn_context.payload.model` (e.g., "gpt-5-codex")

**Schema**:
```jsonl
{"timestamp":"ISO-8601","type":"session_meta","payload":{"id":"UUID","cwd":"/absolute/path","git":{"branch":"main","commit_hash":"..."},"cli_version":"0.53.0",...}}
{"timestamp":"ISO-8601","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"..."}]}}
{"timestamp":"ISO-8601","type":"turn_context","payload":{"cwd":"/absolute/path","model":"gpt-5-codex","approval_policy":"on-request",...}}
```

### 2. Claude Code UUID Format (`claude-code/sample-uuid-session.jsonl`)

**Format**: JSONL (one JSON object per line)

**Key characteristics**:
- Each line is a separate event
- First line may be `file-history-snapshot` with `sessionId: null`
- Main events have `sessionId`, `uuid`, `parentUuid`, and `timestamp`
- Messages contain `isSidechain: false` for main thread
- `version` field indicates Claude Code version (e.g., "2.0.28")
- Tool use via `tool_use` and `tool_result` content types
- Token usage includes cache metrics: `cache_creation_input_tokens`, `cache_read_input_tokens`

**Schema**:
```json
{
  "type": "user|assistant|summary|file-history-snapshot",
  "sessionId": "UUID",
  "uuid": "message-UUID",
  "parentUuid": "UUID|null",
  "isSidechain": false,
  "userType": "external",
  "cwd": "/path/to/project",
  "gitBranch": "branch-name",
  "version": "2.0.28",
  "timestamp": "ISO-8601",
  "message": {
    "role": "user|assistant",
    "content": "string|array",
    "model": "claude-sonnet-4-5-20250929",
    "usage": {
      "input_tokens": 0,
      "output_tokens": 0,
      "cache_creation_input_tokens": 0,
      "cache_read_input_tokens": 0,
      "cache_creation": {
        "ephemeral_5m_input_tokens": 0,
        "ephemeral_1h_input_tokens": 0
      }
    }
  }
}
```

### 3. Claude Code Agent Format (`claude-code/sample-agent-session.jsonl`)

**Format**: JSONL (one JSON object per line)

**Key characteristics**:
- **`isSidechain: true`** - Indicates this is a parallel/sub-task
- **`agentId`** field - Unique identifier for the agent instance
- Shares the same `sessionId` as the parent session
- Typically uses faster models like `claude-haiku-4-5-20251001`
- Smaller file sizes (avg 32KB vs 479KB for UUID format)
- Generated during parallel task execution or agent mode

**Schema**:
```json
{
  "type": "user|assistant",
  "sessionId": "UUID",           // Same as parent session
  "uuid": "message-UUID",
  "parentUuid": "UUID|null",
  "isSidechain": true,           // Key differentiator
  "agentId": "agent-id",         // Unique agent identifier
  "userType": "external",
  "cwd": "/path/to/project",
  "gitBranch": "branch-name",
  "version": "2.0.28",
  "timestamp": "ISO-8601",
  "message": {
    "role": "user|assistant",
    "model": "claude-haiku-4-5-20251001",  // Often Haiku for agents
    "content": [...]
  }
}
```

## Edge Case Fixtures

### Codex Edge Cases (`codex/2025/11/20/rollout-2025-11-20-edge-cases.jsonl`)

Tests the following edge cases:
- **Empty `summary` arrays**: `reasoning` events with `summary: []`
- **Failed function calls**: Function calls with non-zero exit codes (e.g., command not found)
- **Empty `instructions`**: Session with empty instructions string
- **Encrypted content**: `reasoning` events with `encrypted_content` instead of plain `content`

### Claude Code Edge Cases (`edge-cases-session.jsonl`)

Tests the following edge cases:
- **`isMeta: true`**: Meta messages that should be skipped or handled specially
- **`thinkingMetadata`**: User messages with thinking level configuration
- **Error `tool_result`**: Tool results with `is_error: true` and error messages
- **`toolUseResult` with errors**: Error messages in the `toolUseResult` field
- **Large file errors**: Realistic error messages about token limits

### Multi-Snapshot Session (`multi-snapshot-session.jsonl`)

Tests the following scenarios:
- **Multiple `file-history-snapshot` lines**: 3 snapshot events in one session
- **Snapshots before sessionId**: First 2 snapshots have no `sessionId`, testing the "scan all messages" logic
- **`isSnapshotUpdate` variations**: Both `false` (new snapshot) and `true` (update) cases
- **`trackedFileBackups`**: Empty and non-empty file backup tracking

### Agent Multi-Tool Session (`agent-multi-tool-session.jsonl`)

Tests the following scenarios:
- **Multiple consecutive tool calls**: 3 tool calls (Glob → Grep → Read) in sequence
- **Tool call chaining**: Each tool result feeds into the next tool decision
- **Mixed tool types**: Different tools (Glob for finding, Grep for searching, Read for reading)
- **Agent sidechain behavior**: Full agent workflow with `isSidechain: true` and `agentId`

## Parsing Strategy

### Codex Parser
1. Read JSONL file line by line
2. Parse each line as `{timestamp, type, payload}`
3. Extract `cwd` from `session_meta.payload.cwd` (first occurrence)
4. Extract `git` info from `session_meta.payload.git`
5. Extract `model` from `turn_context.payload.model`
6. Map event types:
   - `session_meta` → session metadata (id, cwd, git, instructions)
   - `response_item` → messages/reasoning/tool calls/tool results
   - `turn_context` → context metadata (cwd, model, approval policy)
   - `event_msg` → ignored (internal events)
7. Project path: Use `cwd` from metadata

### Claude Code Parser
1. Read JSONL file line by line
2. **Scan all messages** for first non-null `sessionId` (not just first line!)
3. **Scan all messages** for first non-null `cwd` and `gitBranch`
4. Parse each line as separate event
5. Handle content as either string or array
6. Map `tool_use` and `tool_result` to tool events
7. Extract token usage from `message.usage`

### Key Implementation Notes

- **Codex format**: Extract project path from `session_meta` or `turn_context` `cwd` field
- **Claude Code sessionId extraction**: Must scan all lines because first line may be `file-history-snapshot` with null values
- **Agent files**: Filter by `isSidechain: true` and extract `agentId`
- **Content normalization**: Both string and array formats exist for `message.content`
- **Timestamp handling**: All formats use ISO-8601
- **Token metrics**: Claude Code provides detailed cache metrics, Codex does not

## Usage in Tests

These fixtures can be used to:
1. **Basic schema validation**: Use basic fixtures to test parser correctness
2. **Edge case handling**: Use edge case fixtures to test error handling and optional field parsing
3. **Real-world scenarios**: Use multi-step fixtures to test complex session flows
4. **Schema compatibility**: Verify compatibility across different versions
5. **Metrics computation**: Validate token counting, tool usage tracking, and duration calculations
6. **Robustness testing**: Ensure parsers handle missing/null fields gracefully

## Maintaining Fixtures

When adding new fixtures:
1. Use **dummy data only** - no real session content
2. Maintain realistic schema structure
3. Include edge cases (nulls, empty arrays, etc.)
4. Document any new fields in this README
5. Keep file sizes reasonable (<100KB)
6. **For Codex format**: Include `session_meta` with `cwd` and `git` fields

## Real Data Locations

For reference, real data is stored at:
- **Codex**: `~/.codex/sessions/YYYY/MM/DD/*.jsonl`
- **Claude Code**: `~/.claude/projects/*/*.jsonl`
  - UUID format: `{UUID}.jsonl`
  - Agent format: `agent-{8-char-id}.jsonl`
