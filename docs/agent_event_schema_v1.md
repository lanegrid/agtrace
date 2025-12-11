# agtrace Agent Event v1 Specification

## 0. Scope / Purpose

This specification defines a common normalized schema for agent behavior logs generated from three different tools:

* Claude Code
* Codex (OpenAI Codex CLI logs)
* Gemini CLI

After normalization, the goal is to **handle agent behavior as unified "event units"** across all providers.

* 1 event = 1 "meaningful occurrence"
  * Examples: user message, assistant message, reasoning (thinking), tool call, tool result, file snapshot, etc.
* Vendor-specific formats and granularity differences are absorbed by the normalization logic. This specification aims to be vendor-neutral.

---

## 1. Event Model Overview

### 1.1 Core Principles

* The minimal unit after normalization is **`AgentEventV1`**.
* From each vendor's record, **0 to N `AgentEventV1` events** are generated.
  * Claude: One record may contain multiple tool_use/tool_result/thinking blocks, so it can be **split into multiple events**.
  * Codex: Mostly 1 record = 1 event.
  * Gemini: One session JSON's `messages[]` array is expanded into multiple events.
* Relationships between events are expressed primarily through:
  * `session_id` — conversation/execution session unit
  * `event_id` — event's own ID
  * `parent_event_id` — parent ID indicating "which user_message this belongs to" (see below)
  * `tool_call_id` — matches tool calls with their results

### 1.2 Version

* Schema version: `agtrace.event.v1`
* Normalized events must always have `schema_version = "agtrace.event.v1"`

---

## 2. Type Definitions

### 2.1 Enums

```ts
type Source = "claude_code" | "codex" | "gemini";

type EventType =
  | "user_message"
  | "assistant_message"
  | "system_message"
  | "reasoning"
  | "tool_call"
  | "tool_result"
  | "file_snapshot"
  | "session_summary"
  | "meta"
  | "log";

/**
 * Role indicates who/what is responsible for the event
 *
 * - user: human user
 * - assistant: LLM agent (Claude / Codex / Gemini)
 * - system: system / runtime / IDE
 * - tool: external tool output (bash, apply_patch, editor API, etc.)
 * - cli: CLI user terminal input (e.g. Gemini CLI `/model` command)
 * - other: fallback for cases that don't fit above categories
 */
type Role = "user" | "assistant" | "system" | "tool" | "cli" | "other";

type Channel = "chat" | "editor" | "terminal" | "filesystem" | "system" | "other";

type ToolStatus = "success" | "error" | "in_progress" | "unknown";

type FileOp = "read" | "write" | "modify" | "delete" | "create" | "move";
```

### 2.2 AgentEventV1 Schema

**Logical Model (TypeScript/Pseudo)**

```ts
interface AgentEventV1 {
  // --- Meta Information / Identifiers ---
  schema_version: "agtrace.event.v1";

  /** Original tool type */
  source: Source;

  /**
   * Project hash.
   * Gemini CLI: sha256(projectRoot).hex (following OSS implementation).
   * Claude/Codex: Recommended to calculate from project_root using the same function,
   *               but may have tool-specific definitions, so treat as "project identifier".
   */
  project_hash: string;

  /**
   * Project root path (expected to be absolute path).
   * Claude/Codex: Use cwd or project root.
   * Gemini: Cannot be restored from logs, so set to null.
   */
  project_root: string | null;

  /**
   * Session unit ID.
   * Claude: sessionId
   * Codex: payload.id if exists, otherwise rollout filename, etc.
   * Gemini: sessionId
   */
  session_id: string | null;

  /**
   * Event-specific ID.
   * Claude: Composed from uuid / message.id / tool_use.id, etc.
   * Codex: Prefer vendor ID like payload.call_id.
   * Gemini: messages[].id / messageId, etc.
   */
  event_id: string | null;

  /**
   * ID indicating "which user_message this event belongs to".
   *
   * Rules:
   * - Only events with event_type == "user_message" have parent_event_id = null
   * - All other events have the event_id of the "most recent user_message"
   *   within the same session as parent_event_id.
   *
   * In other words, parent_event_id represents the "root of conversation turn".
   * Note: tool_call and reasoning do NOT have assistant_message as parent.
   */
  parent_event_id: string | null;

  /** RFC3339 UTC timestamp (e.g., "2025-11-26T12:51:28.093Z") */
  ts: string;

  // --- Event Nature ---
  event_type: EventType;
  role: Role | null;
  channel: Channel | null;

  /**
   * Human-readable text.
   * - user_message: User utterance itself
   * - assistant_message: Model response body (first N characters, etc.)
   * - reasoning: Content of thinking/thoughts/reasoning
   * - tool_call: Summary of input arguments
   * - tool_result: stdout or summary
   * - file_snapshot: Brief summary like "snapshot of N files"
   */
  text: string | null;

  // --- Tool / Command Execution ---
  /** Name of called tool/command (e.g., "Bash", "shell", "apply_patch", "Glob") */
  tool_name: string | null;

  /**
   * Tool call ID.
   * Claude: tool_use.id / tool_use_id
   * Codex: payload.call_id
   * Gemini: messages[].toolCalls[].id
   *
   * Key to link tool_call with tool_result.
   */
  tool_call_id: string | null;

  /** Tool execution status */
  tool_status: ToolStatus | null;

  /** Tool execution latency (ms). Fill in only if available. */
  tool_latency_ms: number | null;

  /**
   * Tool exit code (Bash / shell, etc.). null if not available.
   *
   * Extraction method:
   * - Claude: If parsable from toolUseResult
   * - Codex: Extract from FunctionCallOutput's output
   * - Gemini: Extract from result's output text using regex `Exit Code: (\d+)`
   */
  tool_exit_code: number | null;

  // --- File / Code ---
  /**
   * Primary target file path. If multiple, use representative one.
   *
   * For tool calls, recommended to extract from arguments:
   * - Write / write_file / apply_patch: `input.file_path` or `args.file_path`
   * - Read / Glob: `input.file_path` or `input.path`
   * - Edit: `input.file_path`
   */
  file_path: string | null;

  /** Primary language (e.g., "rust", "typescript"). null if cannot be inferred. */
  file_language: string | null;

  /**
   * File operation type.
   *
   * Infer from tool name:
   * - Write / write_file: "write"
   * - Read: "read"
   * - Edit: "modify"
   * - apply_patch: "modify"
   */
  file_op: FileOp | null;

  // --- Model / Tokens ---
  /** Model name used. Claude: message.model, Codex: payload.model, Gemini: messages[].model */
  model: string | null;

  /** Event-level token counts (prefer "single event usage" whenever possible) */
  tokens_input: number | null;
  tokens_output: number | null;
  tokens_total: number | null;
  tokens_cached: number | null;
  tokens_thinking: number | null;
  tokens_tool: number | null;

  /** Vendor-side agent ID (e.g., Claude's agentId) */
  agent_id: string | null;

  /**
   * Field for preserving vendor-specific information.
   *
   * Currently, may store "entire original record" or "close summary".
   * In the future, there's room to introduce size reduction or summarization policy.
   */
  raw: any;
}
```

---

## 3. Relationship Representation (Important Invariants)

### 3.1 parent_event_id

* **Purpose**: Indicates "which user_message this event belongs to".

* Invariants:
  1. Only events with `event_type == "user_message"` have `parent_event_id = null`.
  2. All other events (assistant_message / reasoning / tool_call / tool_result / file_snapshot / meta / log):
     * Within the same `session_id`,
     * Have the `event_id` of the "most recent user_message event" as `parent_event_id`.

* In other words, `parent_event_id` enables **"turn-based" grouping rooted at user messages**.

### 3.2 tool_call_id

* **Purpose**: Indicates "which tool_result corresponds to which tool_call".
* Invariants:
  * Events with `event_type == "tool_call"` and `event_type == "tool_result"` share the same `tool_call_id`.
  * A tool_result with `tool_call_id = null` means "corresponding call cannot be identified" or "aggregated event".

### 3.3 Event Order

* Chronology is primarily tracked by sorting on `ts` (RFC3339).
* If same `ts` and ordering is needed, use original log stream order (consider adding sequence field in implementation if needed).

---

## 4. EventType × Role Mapping Rules (v1.5)

This section defines **mandatory** role assignments for each event_type to eliminate ambiguity and ensure consistency across providers.

### 4.1 Invariant: EventType → Role Mapping

| event_type         | allowed role(s)      | rationale                                                                 |
|--------------------|----------------------|---------------------------------------------------------------------------|
| user_message       | `user`               | Human user input                                                         |
| assistant_message  | `assistant`          | LLM agent response                                                       |
| system_message     | `system`             | System-level messages (e.g., IDE notifications)                          |
| reasoning          | `assistant`          | Agent's internal thinking process                                        |
| tool_call          | `assistant`          | Agent is the entity calling the tool                                     |
| tool_result        | `tool`               | Result returned by the tool itself                                       |
| file_snapshot      | `system`             | IDE/runtime state update                                                 |
| session_summary    | `assistant` \| `system` | Agent's summary → `assistant`; Pure metadata → `system`              |
| meta               | `system`             | Metadata from runtime/system                                             |
| log                | `system` \| `cli`    | Local command logs → `cli`; Runtime logs → `system`                      |

**Key principle:**
- `tool_result` must **always** be `role = "tool"`, not `"assistant"` or `"user"`.
- `reasoning` is always `role = "assistant"` (agent's internal thought).
- `tool_call` is always `role = "assistant"` (agent invokes tools).

This mapping is **enforced** in the normalization layer. Any violation is a spec compliance error.

---

## 5. Provider-Specific Mapping Specifications

This section describes the policy for generating `AgentEventV1` from each vendor's raw logs.

### 5.1 Claude Code

#### 5.1.1 project / session / ID

* `project_root` ← `cwd`
* `project_hash` ← Recommended to use `sha256(project_root)` (to align with Gemini).
  Not mandatory, but follow this rule for consistency.
* `session_id` ← `sessionId`
* `event_id`:
  * user_message: `uuid` or `messageId`
  * assistant-related: `uuid` or `message.id` / `tool_use.id` / composite ID

#### 5.1.2 Event Decomposition

From one Claude record (one JSONL line), generate up to the following events:

* user_message
  * Condition: `message.role == "user"`
  * 1 event per record
* reasoning
  * Condition: `message.content[].type == "thinking"`, etc.
  * 1 event per thinking block
* assistant_message
  * Condition: `message.role == "assistant"` and has text block
  * Usually 1 event per record
* tool_call
  * Condition: `message.content[].type == "tool_use"`
  * 1 event per tool_use
* tool_result
  * Condition:
    * `message.content[].type == "tool_result"`
    * Or when corresponding result exists in `toolUseResult`
  * 1 event per tool_result
* file_snapshot
  * Condition: `type == "file-history-snapshot"`
  * 1 event per record
* session_summary
  * Condition: `type == "summary"`

#### 5.1.3 parent_event_id Assignment

* user_message events:
  * `parent_event_id = null`
* All other events:
  * Set `parent_event_id` to the `event_id` of the "most recent user_message" within the same session

#### 5.1.4 event_type / role / channel / text

**event_type Determination Priority (Important):**

In Claude Code, tool_result may be wrapped with `message.role="user"`, so
**content type must be checked with highest priority**. Apply in this order:

1. If `message.content[]` contains `type == "tool_use"`:
   * `event_type = "tool_call"`
   * `tool_call_id` = that content's `id`

2. If `message.content[]` contains `type == "tool_result"`, or top-level `toolUseResult` exists:
   * `event_type = "tool_result"`
   * `tool_call_id` = that content's `tool_use_id`
   * **Note:** Even if `message.role` is `"user"`, ignore it

3. If `message.content[]` contains `type == "thinking"`:
   * `event_type = "reasoning"`

4. If none of the above:
   * When `message.role == "user"`: `event_type = "user_message"`
   * When `message.role == "assistant"`: `event_type = "assistant_message"`

**role Determination (v1.5 - strictly enforced):**

* `event_type = "user_message"`:
  * `role = "user"`
* `event_type = "assistant_message"`:
  * `role = "assistant"`
* `event_type = "reasoning"`:
  * `role = "assistant"`
* `event_type = "tool_call"`:
  * `role = "assistant"`
* `event_type = "tool_result"`:
  * `role = "tool"` (**override even if `message.role` is `"user"`**)
* `event_type = "file_snapshot"` / `event_type = "meta"`:
  * Principle: `role = "system"`
* `channel`:
  * Normal dialogue: `"chat"`
  * Bash / shell execution (has stdout/stderr): `"terminal"`
  * File read/write / patch: `"editor"` or `"filesystem"`
* `text`:
  * user_message: `message.content` (string or text block)
  * assistant_message: Joined text blocks
  * reasoning: thinking block content
  * tool_call: Summarize `input` (JSON)
  * tool_result: Summary of stdout / content
  * file_snapshot: Brief summary like `"snapshot of N files"`

#### 5.1.5 Tool / Tokens

* `tool_name` ← `message.content[].name` (when type == "tool_use")
* `tool_call_id` ← `message.content[].id` / `.tool_use_id`
* `tool_status`:
  * `toolUseResult.status == "completed"` → `"success"`
  * `interrupted == true` → `"error"`
  * Unknown → `"unknown"`
* `file_path`:
  * tool_call event: `message.content[].input.file_path` (for Write / Read / Edit)
  * tool_result event: `toolUseResult.filePath` / `toolUseResult.file.filePath`, etc.
* `file_op`:
  * Infer from tool name:
    * `Write` → `"write"`
    * `Read` → `"read"`
    * `Edit` → `"modify"`
* `tool_exit_code`:
  * For Bash tool, set if extractable from `toolUseResult`
* `model` ← `message.model`
* `tokens_*` ← `message.usage.*` (prefer single event usage)

---

### 5.2 Codex

#### 5.2.1 project / session / ID

* `project_root` ← `payload.cwd` (if exists)
* `project_hash` ← Recommended to use `sha256(project_root)`
* `session_id`:
  * If `payload.id` exists, use it
  * Otherwise use "rollout filename", etc.
* `event_id`:
  * Tool-related: `payload.call_id`
  * Others: Synthesize with `timestamp + sequence number`, etc.

#### 5.2.2 Event Decomposition

Codex already has 1 record = 1 event structure, so map 1:1 to AgentEventV1.

* `payload.type == "message"`:
  * Covers both user / assistant
* `payload.name` exists:
  * tool_call equivalent (shell, apply_patch, etc.)
* `payload.status` exists:
  * tool_result equivalent
* `payload.type == "reasoning"`:
  * reasoning event
* `payload.type == "token_count"` / `sandbox_policy` / `ghost_commit`, etc.:
  * meta event

#### 5.2.3 parent_event_id

* Message with `payload.role == "user"` is user_message, with `parent_event_id = null`
* Within same `session_id`, subsequent events (assistant_message / reasoning / tool_* / meta):
  * Set `parent_event_id` to the `event_id` of "most recent user_message"

#### 5.2.4 event_type / role / channel / text

* `event_type`:
  * `payload.type == "message" && role == "user"` → `"user_message"`
  * `payload.type == "message" && role == "assistant"` → `"assistant_message"`
  * `payload.type == "reasoning"` → `"reasoning"`
  * `payload.name` exists → `"tool_call"`
  * `payload.status` exists → `"tool_result"`
  * `payload.type in ("token_count", "sandbox_policy", ...)` → `"meta"`
* `role` (v1.5 - strictly enforced):
  * payload.type == "message" && role == "user": `role = "user"`
  * payload.type == "message" && role == "assistant": `role = "assistant"`
  * payload.type == "reasoning" or "agent_reasoning": `role = "assistant"`
  * function_call (tool invocation): `role = "assistant"`
  * function_call_output (tool result): `role = "tool"` (**not** `"assistant"`)
  * meta (token_count, session_meta, etc.): `role = "system"`
* `channel`:
  * tool_call with `name == "shell"` → `"terminal"`
  * tool_call with `name == "apply_patch"` → `"editor"`
  * Other messages → `"chat"`
* `text`:
  * message:
    * Join `payload.content[].text`
    * If not, use `payload.text`
  * tool_call:
    * `payload.arguments` (JSON string) as-is or summarized
  * tool_result:
    * Summarize `payload.output`'s `"output"` portion
  * meta:
    * Generate simple text as needed (e.g., `"sandbox_policy updated"`)

#### 5.2.5 Tool / Tokens

* `tool_name` ← `payload.name`
* `tool_call_id` ← `payload.call_id`
* `tool_status`:
  * `status == "completed"` → `"success"`
  * Otherwise → `"error"` or `"unknown"`
* `file_path`:
  * For function_call / custom_tool_call, JSON parse `arguments` or `input` and look for `file_path` or `path` key
  * Especially important for apply_patch
* `file_op`:
  * `apply_patch` → `"modify"`
  * Other tools inferred from arguments
* `tool_exit_code`:
  * For function_call_output / custom_tool_call_output, attempt regex extraction from `output`
* `model` ← `payload.model`
* `tokens_*`:
  * Use `payload.info.last_token_usage.*` as event-level usage
  * Cumulative usage (`total_token_usage`) kept in raw

---

### 5.3 Gemini CLI

Gemini CLI integrates two types of logs:

1. CLI events (`messageId`, `type`, `message`, `timestamp`, etc.)
2. Session snapshot (`projectHash`, `sessionId`, `messages[]`, etc.)

#### 5.3.1 project / session / ID

* `project_hash`:
  * Use `projectHash` from session snapshot (`logs.json`)
* `project_root`:
  * Cannot be restored from logs, so `null`
* `session_id`:
  * `sessionId`
* `event_id`:
  * CLI events: `String(messageId)`
  * messages[]: `messages[].id`

#### 5.3.2 CLI Single-Line Events

* Condition: Lines with `messageId`, `type`, `message`, `timestamp`
* Mapping:
  * `event_type`: `"user_message"`
  * `role`: `"user"`
  * `channel`: `"cli"`
  * `text`: `message` (e.g., `/model`, `summary this repo`)
  * `parent_event_id`: `null`

#### 5.3.3 Conversation messages[]

Session snapshot contains `messages[]`.
Each `messages[]` element is treated as 1 AgentEvent.

* `messages[].type`:
  * `"user"` → user_message
  * `"gemini"` → assistant_message
  * `"info"` → system_message / meta

* Mapping examples:
  * user_message:
    * `event_type`: `"user_message"`
    * `role`: `"user"`
    * `channel`: `"chat"`
    * `text`: `messages[].content`
    * `parent_event_id`: `null`
  * assistant_message:
    * `event_type`: `"assistant_message"`
    * `role`: `"assistant"`
    * `channel`: `"chat"`
    * `text`: `messages[].content`
    * `model`: `messages[].model`
    * `tokens_input`: `messages[].tokens.input`
    * `tokens_output`: `messages[].tokens.output`
    * `tokens_total`: `messages[].tokens.total`
    * `tokens_cached`: `messages[].tokens.cached`
    * `tokens_thinking`: `messages[].tokens.thoughts`
    * `tokens_tool`: `messages[].tokens.tool`
    * `parent_event_id`: **event_id of most recent user_message**
  * system/info:
    * `event_type`: `"meta"` or `"system_message"`
    * `role`: `"system"`

* reasoning (thoughts array):
  * Each element in `messages[].thoughts[]` becomes one `reasoning` event:
    * `event_type`: `"reasoning"`
    * `role`: `"assistant"`
    * `channel`: `"chat"`
    * `text`: `subject + ": " + description`
    * `parent_event_id`: Corresponding user_message's event_id
      (Or design where `messages[].id` is `parent_event_id` and link to user_message in 2 steps is also acceptable)

* tool_call / tool_result:
  * For each element in `messages[].toolCalls[]`, generate 1 `tool_call` (or tool_call+tool_result) event:
    * tool_call event:
      * `event_type`: `"tool_call"`
      * `role`: `"assistant"` (v1.5 - agent invokes tool)
      * `tool_name`: `name`
      * `tool_call_id`: `id`
      * `text`: Summary of `args`
      * `file_path`: `args.file_path` (for write_file, etc.)
      * `file_op`: Infer from tool name (write_file → "write")
      * `parent_event_id`: Corresponding user_message's event_id
    * tool_result event (when result is included):
      * `event_type`: `"tool_result"`
      * `role`: `"tool"` (v1.5 - tool output, **not** `"assistant"`)
      * `tool_call_id`: `id`
      * `tool_status`: Map `status` to `"success"` / `"error"` / `"unknown"`
      * `text`: Summary of `resultDisplay`
      * `tool_exit_code`: Extract from `result[].functionResponse.response.output` using regex (`Exit Code: (\d+)`)
      * `parent_event_id`: Corresponding user_message's event_id

---

## 6. Versioning and Extensions

* This specification is fixed as `agtrace.event.v1`.
* Breaking changes (field deletion / meaning change) are done by defining `v2`.
* Compatible extensions (field addition / enum value addition) can extend `v1` as-is.

---

## 7. Current Compromises / Notes

* `project_hash`:
  * Ideally use Gemini CLI's `getProjectHash(projectRoot)` implementation reference as "sha256(hex) of project_root", but
  * Claude / Codex's `cwd` may not always point to same project_root,
  * So treat as "project identifier".
* `raw`:
  * Currently acceptable to store entire original record (size is a separate concern, compromised for now).
  * In future, there's room to introduce "summarize and store large content" policy.
* `parent_event_id`:
  * Explicitly limited to "pointer to user_message".
    Causality of tool_call → tool_result is traced via `tool_call_id` + `ts`.
* **Claude's `isSidechain` flag**:
  * Claude Code logs have `isSidechain: true` flag indicating "preparation work" or "behind-the-scenes thinking" that doesn't need to be shown to user.
  * v1 doesn't have dedicated field, preserved as `raw.isSidechain`.
  * UI / analysis side recommended to check `raw.isSidechain == true` for filtering.
  * Future v1.1 may consider adding `is_internal: bool` field.

---

This concludes the `AgentEventV1` specification and mapping specifications for Claude / Codex / Gemini logs.
In implementation, build a normalization layer that generates "1 vendor record → 0 to N AgentEventV1" according to this specification.
