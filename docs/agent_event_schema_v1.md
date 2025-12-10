# agtrace Agent Event v1 Specification

## 0. Scope / Purpose

本仕様は、以下 3 つのツールから生成されるエージェント行動ログを共通フォーマットに正規化するためのスキーマ仕様である。

* Claude Code
* Codex（OpenAI Codex CLI ログ）
* Gemini CLI

正規化後は、**エージェントの行動を「イベント単位」で統一的に扱えること**を目的とする。

* 1 イベント = 1 つの「意味のある事象」

  * 例: user メッセージ、assistant メッセージ、reasoning（思考）、tool 呼び出し、tool の結果、ファイルスナップショットなど
* 「ベンダー固有のフォーマット」や「ログの粒度の違い」は、正規化ロジックで吸収し、本仕様は vendor-neutral であることを目指す。

---

## 1. イベントモデル概要

### 1.1 基本原則

* 正規化後の最小単位は **`AgentEventV1`**。
* 各ベンダーの 1 レコードから **0〜N 個の `AgentEventV1`** が生成される。

  * Claude: 1 レコードに複数の tool_use / tool_result / thinking が入っているため、**複数イベントに分解**されうる。
  * Codex: ほぼ 1 レコード = 1 イベント。
  * Gemini: 1 セッション JSON の `messages[]` を展開して複数イベント化する。
* イベント間の関係は、主に次の 3 つで表現する:

  * `session_id` … 会話 / 実行セッション単位
  * `event_id` … イベント自身の ID
  * `parent_event_id` … 「どの user_message に属するか」の親 ID（後述）
  * `tool_call_id` … ツール呼び出しと結果の対応付け

### 1.2 バージョン

* スキーマバージョン: `agtrace.event.v1`
* 正規化したイベントには必ず `schema_version = "agtrace.event.v1"` を設定する。

---

## 2. 型定義

### 2.1 列挙型

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
```

### 2.2 AgentEventV1 スキーマ

**論理モデル（TypeScript/Pseudo）**

```ts
interface AgentEventV1 {
  // --- メタ情報 / 識別子 ---
  schema_version: "agtrace.event.v1";

  /** 元のツール種別 */
  source: Source;

  /**
   * プロジェクトのハッシュ。
   * Gemini CLI: projectRoot に対する sha256(projectRoot).hex（OSS 実装準拠）。
   * Claude/Codex: project_root から同じ関数で計算することを推奨するが、
   *               ツールごとに異なる定義を持つ可能性があるため、あくまで「プロジェクト識別子」として扱う。
   */
  project_hash: string;

  /**
   * プロジェクトのルートパス（絶対パス想定）。
   * Claude/Codex: cwd やプロジェクトルートを入れる。
   * Gemini: ログからは復元できないため null とする。
   */
  project_root: string | null;

  /**
   * セッション単位の ID。
   * Claude: sessionId
   * Codex: payload.id があればそれ、なければ rollout ファイル名など。
   * Gemini: sessionId
   */
  session_id: string | null;

  /**
   * イベント固有の ID。
   * Claude: uuid / message.id / tool_use.id などから合成。
   * Codex: payload.call_id など、vendor の ID を優先。
   * Gemini: messages[].id / messageId など。
   */
  event_id: string | null;

  /**
   * 「どの user_message に属するイベントか」を示す ID。
   *
   * ルール:
   * - event_type == "user_message" のイベントだけ parent_event_id = null
   * - それ以外のイベントは、同一セッション内で「直近の user_message」の event_id を parent_event_id に持つ。
   *
   * つまり parent_event_id は「会話ターンのルート」を表す。
   * tool_call や reasoning の親が assistant_message になるわけではない点に注意。
   */
  parent_event_id: string | null;

  /** RFC3339 UTC タイムスタンプ (例: "2025-11-26T12:51:28.093Z") */
  ts: string;

  // --- イベントの性質 ---
  event_type: EventType;
  role: Role | null;
  channel: Channel | null;

  /**
   * 人間向けに読めるテキスト。
   * - user_message: ユーザー発話そのもの
   * - assistant_message: モデル応答の本文（先頭 N 文字など）
   * - reasoning: thinking / thoughts / reasoning の本文
   * - tool_call: 入力引数の要約
   * - tool_result: stdout や要約
   * - file_snapshot: snapshot の要約
   */
  text: string | null;

  // --- ツール / コマンド実行 ---
  /** 呼び出されたツール名 / コマンド名 (例: "Bash", "shell", "apply_patch", "Glob") */
  tool_name: string | null;

  /**
   * ツール呼び出しの ID。
   * Claude: tool_use.id / tool_use_id
   * Codex: payload.call_id
   * Gemini: messages[].toolCalls[].id
   *
   * tool_call と tool_result を紐付けるためのキー。
   */
  tool_call_id: string | null;

  /** ツール実行ステータス */
  tool_status: ToolStatus | null;

  /** ツール実行レイテンシ (ms)。取れる場合のみ埋める。 */
  tool_latency_ms: number | null;

  /**
   * ツールの exit code（Bash / shell 等）。なければ null。
   *
   * 抽出方法:
   * - Claude: toolUseResult からパース可能な場合
   * - Codex: FunctionCallOutput の output から抽出
   * - Gemini: result の output テキストから正規表現 `Exit Code: (\d+)` で抽出
   */
  tool_exit_code: number | null;

  // --- ファイル / コード ---
  /**
   * 主に対象となったファイルパス。複数ある場合は代表 1 件。
   *
   * ツール呼び出しの場合、引数から抽出することを推奨:
   * - Write / write_file / apply_patch: `input.file_path` または `args.file_path`
   * - Read / Glob: `input.file_path` または `input.path`
   * - Edit: `input.file_path`
   */
  file_path: string | null;

  /** 主な言語（例: "rust", "typescript"）。推定できなければ null。 */
  file_language: string | null;

  /**
   * ファイル操作種別。必要に応じて Enum 化を検討。
   *
   * ツール名から推論:
   * - Write / write_file: "write"
   * - Read: "read"
   * - Edit: "modify"
   * - apply_patch: "modify"
   */
  file_op: "read" | "write" | "modify" | "delete" | "create" | "move" | null;

  // --- モデル / トークン ---
  /** 使用モデル名。Claude: message.model, Codex: payload.model, Gemini: messages[].model */
  model: string | null;

  /** イベント単位のトークン数（可能な限り「単イベントの usage」を入れる） */
  tokens_input: number | null;
  tokens_output: number | null;
  tokens_total: number | null;
  tokens_cached: number | null;
  tokens_thinking: number | null;
  tokens_tool: number | null;

  /** ベンダー側の agent ID（Claude の agentId 等） */
  agent_id: string | null;

  /**
   * ベンダー固有情報の保持用フィールド。
   *
   * 現時点では「元レコード全体」または「それに近いサマリ」を格納してよい。
   * 将来的にはサイズ削減やサマリ化ポリシーを導入する余地がある。
   */
  raw: any;
}
```

---

## 3. 関係性の表現（重要な不変条件）

### 3.1 parent_event_id

* **目的**: 「どの user_message に対するイベントか」を表す。

* 不変条件:

  1. `event_type == "user_message"` のイベントだけ `parent_event_id = null`。
  2. それ以外のイベント（assistant_message / reasoning / tool_call / tool_result / file_snapshot / meta / log）は、

     * 同じ `session_id` 内で、
     * 「直近の user_message イベント」の `event_id` を `parent_event_id` に持つ。

* つまり、`parent_event_id` によって **ユーザーメッセージを根とする「ターン単位」のグルーピング**ができる。

### 3.2 tool_call_id

* **目的**: 「どの tool_result が、どの tool_call に対応しているか」を表す。
* 不変条件:

  * `event_type == "tool_call"` と `event_type == "tool_result"` のイベントは、同じ `tool_call_id` を共有する。
  * `tool_call_id` が null の tool_result は「対応する call が特定できない」か「まとめイベント」である。

### 3.3 イベント順序

* 時系列は基本的に `ts`（RFC3339）でソートして追う。
* 同一 `ts` かつ順序が必要な場合は、元ログのストリーム順を利用する（実装側で sequence を持ちたい場合は別途フィールド追加を検討）。

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

## 5. ベンダー別マッピング仕様

この章では、各ベンダーの生ログから `AgentEventV1` を生成する際の方針を示す。

### 5.1 Claude Code

#### 5.1.1 project / session / ID

* `project_root` ← `cwd`
* `project_hash` ← `sha256(project_root)` を推奨（Gemini と合わせたい場合）。
  必須ではないが、一貫性のためこのルールに従う。
* `session_id` ← `sessionId`
* `event_id`:

  * user_message: `uuid` または `messageId`
  * assistant 系: `uuid` または `message.id` / `tool_use.id` / 合成 ID

#### 5.1.2 イベント分解

Claude の 1 レコード（JSONL 1 行）から、最大で以下のイベントを生成する。

* user_message

  * 条件: `message.role == "user"`
  * 1 レコードにつき 1 イベント
* reasoning

  * 条件: `message.content[].type == "thinking"` など
  * 各 thinking ブロックごとに 1 イベント
* assistant_message

  * 条件: `message.role == "assistant"` かつ text ブロックあり
  * 通常 1 レコードにつき 1 イベント
* tool_call

  * 条件: `message.content[].type == "tool_use"`
  * 各 tool_use ごとに 1 イベント
* tool_result

  * 条件:

    * `message.content[].type == "tool_result"`
    * または `toolUseResult` に対応する結果が存在する場合
  * 各 tool_result ごとに 1 イベント
* file_snapshot

  * 条件: `type == "file-history-snapshot"`
  * 1 レコードにつき 1 イベント
* session_summary

  * 条件: `type == "summary"`

#### 5.1.3 parent_event_id の付与

* user_message イベント:

  * `parent_event_id = null`
* その他すべてのイベント:

  * 同一セッション内で、「直近の user_message」の `event_id` を `parent_event_id` に設定

#### 5.1.4 event_type / role / channel / text

**event_type 判定の優先順位（重要）:**

Claude Code では、tool_result が `message.role="user"` でラップされることがあるため、
**content の type を最優先で判定**する必要がある。以下の順序で適用：

1. `message.content[]` に `type == "tool_use"` を含む場合:
   * `event_type = "tool_call"`
   * `tool_call_id` = 該当コンテンツの `id`

2. `message.content[]` に `type == "tool_result"` を含む場合、または top-level に `toolUseResult` が存在する場合:
   * `event_type = "tool_result"`
   * `tool_call_id` = 該当コンテンツの `tool_use_id`
   * **注:** この場合、`message.role` が `"user"` であっても無視する

3. `message.content[]` に `type == "thinking"` を含む場合:
   * `event_type = "reasoning"`

4. 上記いずれにも該当しない場合:
   * `message.role == "user"` のとき `event_type = "user_message"`
   * `message.role == "assistant"` のとき `event_type = "assistant_message"`

**role 判定 (v1.5 - strictly enforced):**

* `event_type = "user_message"`:
  * `role = "user"`
* `event_type = "assistant_message"`:
  * `role = "assistant"`
* `event_type = "reasoning"`:
  * `role = "assistant"`
* `event_type = "tool_call"`:
  * `role = "assistant"`
* `event_type = "tool_result"`:
  * `role = "tool"` (**`message.role` が `"user"` であっても override する**)
* `event_type = "file_snapshot"` / `event_type = "meta"`:
  * 原則として `role = "system"`
* `channel`:

  * 通常の対話: `"chat"`
  * Bash / shell 実行（stdout/stderr を持つ）: `"terminal"`
  * ファイル read/write / patch: `"editor"` または `"filesystem"`
* `text`:

  * user_message: `message.content`（string または text ブロック）
  * assistant_message: text ブロックを join したもの
  * reasoning: thinking ブロック本文
  * tool_call: `input`（JSON）をサマライズ
  * tool_result: stdout / content のサマリ
  * file_snapshot: `"snapshot of N files"` のような短い要約

#### 5.1.5 ツール / トークン

* `tool_name` ← `message.content[].name`（type == "tool_use"）
* `tool_call_id` ← `message.content[].id` / `.tool_use_id`
* `tool_status`:

  * `toolUseResult.status == "completed"` → `"success"`
  * `interrupted == true` → `"error"`
  * 不明 → `"unknown"`
* `file_path`:

  * tool_call イベント: `message.content[].input.file_path`（Write / Read / Edit の場合）
  * tool_result イベント: `toolUseResult.filePath` / `toolUseResult.file.filePath` など
* `file_op`:

  * ツール名から推論:
    * `Write` → `"write"`
    * `Read` → `"read"`
    * `Edit` → `"modify"`
* `tool_exit_code`:

  * Bash ツールの場合、`toolUseResult` から抽出可能なら設定
* `model` ← `message.model`
* `tokens_*` ← `message.usage.*`（単イベントの usage を優先）

---

### 5.2 Codex

#### 5.2.1 project / session / ID

* `project_root` ← `payload.cwd`（存在する場合）
* `project_hash` ← `sha256(project_root)` を推奨
* `session_id`:

  * `payload.id` が存在すればそれ
  * なければ「rollout ファイル名」など
* `event_id`:

  * tool 関連: `payload.call_id`
  * それ以外: `timestamp + 通し番号` 等で合成してもよい

#### 5.2.2 イベント分解

Codex はすでに 1 レコード = 1 イベントの構造になっているので、そのまま 1→1 で AgentEventV1 に対応させる。

* `payload.type == "message"`:

  * user / assistant の両方をカバー
* `payload.name` がある:

  * tool_call 相当（shell, apply_patch 等）
* `payload.status` がある:

  * tool_result 相当
* `payload.type == "reasoning"`:

  * reasoning イベント
* `payload.type == "token_count"` / `sandbox_policy` / `ghost_commit` など:

  * meta イベント

#### 5.2.3 parent_event_id

* `payload.role == "user"` の message を user_message とし、`parent_event_id = null`
* 同じ `session_id` 内で、それ以降のイベント（assistant_message / reasoning / tool_* / meta）は、

  * 「直近の user_message」の `event_id` を `parent_event_id` に設定

#### 5.2.4 event_type / role / channel / text

* `event_type`:

  * `payload.type == "message" && role == "user"` → `"user_message"`
  * `payload.type == "message" && role == "assistant"` → `"assistant_message"`
  * `payload.type == "reasoning"` → `"reasoning"`
  * `payload.name` あり → `"tool_call"`
  * `payload.status` あり → `"tool_result"`
  * `payload.type in ("token_count", "sandbox_policy", ...)` → `"meta"`
* `role` (v1.5 - strictly enforced):

  * payload.type == "message" && role == "user": `role = "user"`
  * payload.type == "message" && role == "assistant": `role = "assistant"`
  * payload.type == "reasoning" or "agent_reasoning": `role = "assistant"`
  * function_call (tool invocation): `role = "assistant"`
  * function_call_output (tool result): `role = "tool"` (**not** `"assistant"`)
  * meta (token_count, session_meta, etc.): `role = "system"`
* `channel`:

  * tool_call `name == "shell"` → `"terminal"`
  * tool_call `name == "apply_patch"` → `"editor"`
  * それ以外の message → `"chat"`
* `text`:

  * message:

    * `payload.content[].text` を join
    * なければ `payload.text`
  * tool_call:

    * `payload.arguments`（JSON string）をそのまま or サマライズ
  * tool_result:

    * `payload.output` の `"output"` 部分をサマライズ
  * meta:

    * 必要に応じて簡易テキストを生成（例: `"sandbox_policy updated"`）

#### 5.2.5 ツール / トークン

* `tool_name` ← `payload.name`
* `tool_call_id` ← `payload.call_id`
* `tool_status`:

  * `status == "completed"` → `"success"`
  * その他 → `"error"` or `"unknown"`
* `file_path`:

  * function_call / custom_tool_call の場合、`arguments` または `input` を JSON パースして `file_path` または `path` キーを探す
  * apply_patch の場合は特に重要
* `file_op`:

  * `apply_patch` → `"modify"`
  * 他のツールは引数から推測
* `tool_exit_code`:

  * function_call_output / custom_tool_call_output の `output` から正規表現で抽出を試みる
* `model` ← `payload.model`
* `tokens_*`:

  * イベント単位の usage として `payload.info.last_token_usage.*` を利用
  * 累積 usage (`total_token_usage`) は raw に残す

---

### 5.3 Gemini CLI

Gemini CLI は 2 種類のログを統合して扱う:

1. CLI イベント（`messageId`, `type`, `message`, `timestamp` 等）
2. セッションスナップショット（`projectHash`, `sessionId`, `messages[]` など）

#### 5.3.1 project / session / ID

* `project_hash`:

  * セッションスナップショット (`logs.json`) の `projectHash` を使う
* `project_root`:

  * ログからは復元できないため `null`
* `session_id`:

  * `sessionId`
* `event_id`:

  * CLI イベント: `String(messageId)`
  * messages[]: `messages[].id`

#### 5.3.2 CLI 1 行イベント

* 条件: `messageId`, `type`, `message`, `timestamp` を持つ行
* マッピング:

  * `event_type`: `"user_message"`
  * `role`: `"user"`
  * `channel`: `"cli"`
  * `text`: `message`（例: `/model`, `summary this repo`）
  * `parent_event_id`: `null`

#### 5.3.3 会話 messages[]

セッションスナップショットには `messages[]` が含まれる。
各 `messages[]` 要素を 1 AgentEvent として扱う。

* `messages[].type`:

  * `"user"` → user_message
  * `"gemini"` → assistant_message
  * `"info"` → system_message / meta

* マッピング例:

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
    * `parent_event_id`: **直近の user_message の event_id**
  * system/info:

    * `event_type`: `"meta"` または `"system_message"`
    * `role`: `"system"`

* reasoning（thoughts 配列）:

  * `messages[].thoughts[]` の各要素は、それぞれ 1 つの `reasoning` イベントにする。

    * `event_type`: `"reasoning"`
    * `role`: `"assistant"`
    * `channel`: `"chat"`
    * `text`: `subject + ": " + description`
    * `parent_event_id`: 対応する user_message の event_id
      （もしくは `messages[].id` を `parent_event_id` とし、user_message への紐付けは 2段階で辿る設計も許容）

* tool_call / tool_result:

  * `messages[].toolCalls[]` の各要素について 1 `tool_call`（または tool_call+tool_result）イベントを生成:

    * tool_call イベント:
      * `event_type`: `"tool_call"`
      * `role`: `"assistant"` (v1.5 - agent invokes tool)
      * `tool_name`: `name`
      * `tool_call_id`: `id`
      * `text`: `args` のサマリ
      * `file_path`: `args.file_path`（write_file 等の場合）
      * `file_op`: ツール名から推論（write_file → "write"）
      * `parent_event_id`: 対応する user_message の event_id
    * tool_result イベント（result が含まれている場合）:
      * `event_type`: `"tool_result"`
      * `role`: `"tool"` (v1.5 - tool output, **not** `"assistant"`)
      * `tool_call_id`: `id`
      * `tool_status`: `status` を `"success"` / `"error"` / `"unknown"` にマップ
      * `text`: `resultDisplay` のサマリ
      * `tool_exit_code`: `result[].functionResponse.response.output` から正規表現で抽出（`Exit Code: (\d+)`）
      * `parent_event_id`: 対応する user_message の event_id

---

## 6. バージョニングと拡張

* 本仕様は `agtrace.event.v1` として固定する。
* 互換性の壊れる変更（フィールド削除 / 意味の変化）は `v2` を定義して行う。
* 互換性の保たれる拡張（フィールド追加 / enum 値追加）は、`v1` のまま拡張可能。

---

## 7. 現時点での割り切り / 注意点

* `project_hash`:

  * Gemini CLI の `getProjectHash(projectRoot)` 実装を参考に、「project_root の sha256(hex)」とするのが理想だが、
  * Claude / Codex の `cwd` が必ず同じ project_root を指す保証はないため、
  * あくまで「プロジェクト識別子」として扱う。
* `raw`:

  * 現時点では元レコード全体を格納してもよい（サイズは別問題として今は割り切る）。
  * 将来的には「巨大な content をサマライズして格納する」などの方針を導入する余地あり。
* `parent_event_id`:

  * 明示的に「user_message へのポインタ」として用途を限定している。
    tool_call → tool_result の因果関係は `tool_call_id` + `ts` で辿る。
* **Claude の `isSidechain` フラグ**:

  * Claude Code のログには `isSidechain: true` というフラグがあり、ユーザーに見せる必要のない「準備運動」や「裏側の思考プロセス」を示す。
  * v1 では専用フィールドを持たず、`raw.isSidechain` として保持する。
  * UI / 分析側で `raw.isSidechain == true` をチェックしてフィルタリングすることを推奨。
  * 将来的に v1.1 で `is_internal: bool` などのフィールド追加を検討。

---

以上が `AgentEventV1` の仕様および、Claude / Codex / Gemini 各ログからのマッピング仕様である。
実装では、本仕様に沿って「1 ベンダーレコード → 0〜N AgentEventV1」を生成する正規化レイヤーを構築する。
