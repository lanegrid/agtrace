# Claude Code log format changes (v2.1.x / Opus 4.7–4.8)

Audit of how the Claude Code JSONL transcript format diverged from what the
`agtrace-providers` Claude parser (`crates/agtrace-providers/src/claude/`) was
originally written against. Captured 2026-06 from ~40k records across recent
sessions (Claude Code v2.1.119–v2.1.159, models `claude-opus-4-7` / `-4-8`).

All files still **parse without error** (serde ignores unknown fields and
`#[serde(other)]` catches unknown record types), so nothing crashes. The
problem is that the new format **silently drops information**: new record types
fall into `Unknown`, and `thinking` text is no longer present.

## Summary

| # | Area | Severity | Status |
|---|------|----------|--------|
| 1 | `thinking` text now empty (signature only) | medium | empty Reasoning events render blank |
| 2 | 7 new record types fall into `Unknown` | high | ai-title / custom-title / last-prompt / attachment / mode / permission-mode / agent-name |
| 3 | `agentId` removed from all records | high | subagent / sidechain attribution can no longer key on it |
| 4 | new `system` subtypes unhandled | medium | away_summary / api_error / informational |
| 5 | new `usage` fields unused | medium | thinking_tokens (= reasoning), iterations, server_tool_use |
| 6 | assistant API-error records unhandled | medium | isApiErrorMessage / error / apiErrorStatus |
| 7 | 13+ `attachment` subtypes dropped | medium | plan_mode / skill_listing / task_reminder, etc. |
| 8 | new `message` fields uncaptured | low | stop_details / diagnostics / context_management / container |

## 1. Record types

Observed `.type` values (counts from the sampled scan):

```
assistant 14355 / user 9159 / last-prompt 2043 / pr-link 1810 / ai-title 1659
file-history-snapshot 1458 / attachment 1118 / system 845 / mode 740
permission-mode 444 / custom-title 419 / queue-operation 330 / agent-name 75
```

Mapping against the current `ClaudeRecord` enum (`schema.rs`):

| Current variant | State |
|---|---|
| FileHistorySnapshot / User / Assistant / System / QueueOperation / PrLink | present (but with new fields, see below) |
| Progress | **absent in new logs** (legacy; subagent progress moved elsewhere) |
| Summary | absent in new logs (superseded by the title records) |
| Unknown (`#[serde(other)]`) | absorbs the 7 new types below and discards them |

New record types not yet modeled (currently `Unknown` → discarded):

| type | fields | meaning |
|---|---|---|
| `ai-title` | aiTitle, sessionId | AI-generated session title |
| `custom-title` | customTitle, sessionId | user-set session title |
| `last-prompt` | lastPrompt, leafUuid, sessionId | most recent user prompt (full text) |
| `agent-name` | agentName, sessionId | named agent for the session |
| `mode` | mode (`normal`, …) | output mode toggle |
| `permission-mode` | permissionMode (`default`/`acceptEdits`/`auto`/`plan`) | permission mode |
| `attachment` | attachment{…}, cwd, gitBranch, … | rich attachment payloads (§7) |

## 2. `thinking` text removed (the original symptom)

Every `thinking` content block in the new logs looks like:

```json
{"type":"thinking","thinking":"","signature":"ErEC…(encrypted)"}
```

- `thinking` is **always an empty string** — zero non-empty across all sampled
  opus-4-7/4-8 blocks. The legacy fixture (`claude-3-5`) carried full text.
- Only the encrypted `signature` remains (sent back to the API for context
  continuity). **The plaintext cannot be recovered** — this is an upstream
  Claude Code change for "adaptive thinking".
- `parser.rs` currently turns the empty string into a `Reasoning` event, which
  renders as a blank `Reasoning:` line.
- Plan: detect empty-`thinking`-with-`signature` and emit a redacted marker
  (e.g. `[thinking redacted]`); optionally annotate with `thinking_tokens` (§5).

## 3. `agentId` removed → subagent / sidechain attribution

- `agentId` is **gone** from user, assistant, and tool_result records.
- `parser.rs::determine_stream_id()` keys `StreamId::Sidechain` off `agent_id`,
  so on the new format it always falls back to `unknown`.
- `UserContent::ToolResult.agentId`, `tool_use_result.agentId`, and
  `AssistantRecord.agent_id` are no longer populated (parse still succeeds via
  serde defaults, values are just absent).
- No session in the available data uses subagents (`isSidechain:true` and
  `name:"Task"` both occur 0 times), so the **new** linking mechanism cannot be
  reverse-engineered from current data. Candidates to investigate when subagent
  logs become available: `agent-name` records, user `sourceToolUseID` /
  `sourceToolAssistantUUID`, and the tool_use `caller` field.
- **Deferred** until subagent transcripts exist. The existing `agentId`-based
  path is kept as a harmless fallback.

## 4. `system` record: new subtypes / fields

Observed subtypes: `turn_duration`, `away_summary`, `stop_hook_summary`,
`local_command`, `api_error`, `informational`, `compact_boundary`.

Handled today: `local_command`, `turn_duration`, `compact_boundary`,
`stop_hook_summary`. Unhandled:

- `away_summary` — `content` holds a free-text summary of the work (useful).
- `api_error` — `retryInMs` / `retryAttempt` / `maxRetries` / `content`
  (retry visibility).
- `informational` — generic info message.

New `system` fields: messageCount, toolUseID, stopReason, hookErrors, hasOutput,
retryInMs, retryAttempt, maxRetries, error, logicalParentUuid.

## 5. `usage` (token accounting): new fields

Observed keys: input_tokens, output_tokens, cache_creation_input_tokens,
cache_read_input_tokens, cache_creation, **iterations**,
**output_tokens_details**, **server_tool_use**, service_tier, inference_geo,
speed.

- `output_tokens_details.thinking_tokens` = the reasoning token count. The
  parser currently hardcodes `reasoning = 0` (`parser.rs`); this fills it.
  Note `TokenOutput.total() = generated + reasoning + tool`, and the API's
  `output_tokens` already includes thinking, so `generated` must be
  `output_tokens - thinking_tokens` to avoid double counting.
- `iterations` (array, usually length 1) = adaptive-thinking inference passes.
- `server_tool_use` = web_search_requests / web_fetch_requests counts.
- service_tier / inference_geo / speed = metadata.

Top-level input/output/cache_* are unchanged, so existing token math still
works.

## 6. `attachment` subtypes (all dropped today)

| subtype | fields | count |
|---|---|---|
| task_reminder | content, itemCount | 855 |
| command_permissions | allowedTools | 73 |
| skill_listing | names, skillCount, isInitial | 48 |
| edited_text_file | filename, snippet | 45 |
| deferred_tools_delta | addedNames | 41 |
| queued_command | prompt, commandMode | 23 |
| date_change | — | 17 |
| file | — | 9 |
| plan_mode | isSubAgent, planExists, planFilePath, reminderType | 2 |
| plan_mode_exit | planExists, planFilePath | 2 |
| plan_mode_reentry | — | 1 |
| invoked_skills | skills | 2 |
| compact_file_reference | displayPath, filename | 1 |

The legacy `UserRecord.plan_content` plan-mode tracking has moved to
`attachment.plan_mode*` plus a plan-file reference. Skill invocation is now
tracked via `invoked_skills` / `skill_listing`.

## 7. New fields on existing records

### assistant (top-level)
`attributionSkill`, `entrypoint`, `slug`, and the API-error trio
`isApiErrorMessage` / `error` / `apiErrorStatus` (API errors are recorded as
assistant records).

### assistant.message
`stop_details`, `diagnostics`, `context_management`, `container`.

### assistant.message.content[tool_use]
`caller` (e.g. `{"type":"direct"}`).

### user (top-level)
New: `promptId`, `permissionMode`, `origin` (e.g. `{"kind":"task-notification"}`),
`sourceToolUseID`, `sourceToolAssistantUUID`, `interruptedMessageId`,
`isVisibleInTranscriptOnly`, `entrypoint`.
Removed: `agentId`, `planContent`, `thinkingMetadata` (legacy fields still in the
struct but no longer populated).

## Roadmap

- **A. Parsing foundation (high):** model the 7 new record types; emit events
  for the high-value ones, explicitly skip the rest.
- **B. Subagent / sidechain linking (high, deferred):** identify the new
  mechanism once subagent transcripts exist; redesign `determine_stream_id`.
- **C. Redacted thinking (medium):** marker for empty thinking + `thinking_tokens`.
- **D. Usage enrichment (medium):** `thinking_tokens` → reasoning, `server_tool_use`.
- **E. New `system` subtypes (medium):** away_summary / api_error / informational.
- **F. Attachments (medium):** selectively surface plan_mode / skill_listing /
  task_reminder, etc.
- **G. Metadata (low):** ai-title / custom-title as session titles;
  permission-mode / mode visibility.

Each item can ship as an independent PR. New-format snapshot fixtures are a
prerequisite.
