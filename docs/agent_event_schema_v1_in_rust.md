```rust
//! Normalized agent event schema (agtrace.event.v1)
//!
//! このモジュールは、Claude Code / Codex / Gemini CLI のログを統一形式に正規化するための
//! スキーマおよびマッピング仕様を Rust の型として定義する。
//!
//! - 1 ベンダーログレコードから 0〜N 個の `AgentEventV1` を生成することを前提とする。
//! - 各フィールドのドキュメントコメントに、ベンダー別のマッピング方針を明記している。
//!
//! 実際の正規化ロジック（パーサ/トランスフォーマー）はこの型に合わせて実装する。

use serde::{Deserialize, Serialize};

/// 生ログの出所（ベンダー / ツール種別）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// Claude Code (Anthropic)
    ClaudeCode,
    /// Codex CLI rollout logs
    Codex,
    /// Gemini CLI
    Gemini,
}

/// 正規化済みイベントの種別
///
/// - 「何が起きたイベントなのか」の抽象カテゴリ
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// ユーザーからのメッセージ
    ///
    /// - Claude: `message.role == "user"`
    /// - Codex : `payload.type == "message" && payload.role == "user"`
    /// - Gemini: `messages[].type == "user"` または CLI の `/...` 入力
    UserMessage,

    /// アシスタント（LLM）からの自然言語メッセージ
    ///
    /// - Claude: `message.role == "assistant"` の text 部分
    /// - Codex : `payload.type == "message" && payload.role == "assistant"`
    /// - Gemini: `messages[].type == "gemini"`
    AssistantMessage,

    /// システム / 情報メッセージ
    ///
    /// - Gemini: `messages[].type == "info"`
    /// - Codex : system 向けの `event_msg` 等（必要に応じて使用）
    SystemMessage,

    /// 内部推論（reasoning / thinking / thoughts）イベント
    ///
    /// - Claude: `message.content[].type == "thinking"`
    /// - Codex : `payload.type == "reasoning"`
    /// - Gemini: `messages[].thoughts[]`
    Reasoning,

    /// ツール呼び出し（Bash / apply_patch / 読み取りツール等）
    ///
    /// - Claude: `message.content[].type == "tool_use"`
    /// - Codex : `payload.name` が存在（shell, apply_patch など）
    /// - Gemini: `messages[].toolCalls[]`
    ToolCall,

    /// ツールの結果（stdout / ファイル差分 / 成否など）
    ///
    /// - Claude: `message.content[].type == "tool_result"` や `toolUseResult`
    /// - Codex : `payload.status` が存在 + `payload.output`
    /// - Gemini: `toolCalls[].result` 相当
    ToolResult,

    /// ファイルスナップショット（バックアップ・歴史情報）
    ///
    /// - Claude: `type == "file-history-snapshot"` および `snapshot.*`
    FileSnapshot,

    /// セッション要約
    ///
    /// - Claude: `type == "summary"` レコード
    /// - Gemini: session 単位のサマリとして生成する場合に使用
    SessionSummary,

    /// その他のメタ情報
    ///
    /// - Codex : token_count, sandbox_policy, rate_limits など
    /// - Gemini: セッションメタデータなど
    Meta,

    /// ログ / ローカルコマンドメッセージなど
    ///
    /// - Claude: `subtype == "local_command"` 系など、対話というよりログ的な行
    Log,
}

/// 発話 / 実行主体
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    System,
    /// 純粋な tool / command として扱いたい場合
    Tool,
    /// 端末 CLI からの入力
    Cli,
    Other,
}

/// イベントが属するチャネル
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// 通常のチャット（プロンプト / 回答）
    Chat,
    /// エディタ操作（コード編集 / patch）
    Editor,
    /// ターミナル / shell 実行
    Terminal,
    /// ファイルシステム操作（read/write/delete 等）
    Filesystem,
    /// システムメッセージ / メタデータ
    System,
    Other,
}

/// ツール実行ステータス
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    Success,
    Error,
    InProgress,
    Unknown,
}

/// 正規化された 1 イベント
///
/// 1 vendor レコードから 0〜N 個の `AgentEventV1` が生成される。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEventV1 {
    /// スキーマバージョン。常に "agtrace.event.v1"。
    pub schema_version: String,

    /// 元のツール種別（Claude Code / Codex / Gemini CLI）。
    pub source: Source,

    // -------------------------------------------------------------------------
    // プロジェクト / セッション / ID 系
    // -------------------------------------------------------------------------

    /// プロジェクト識別子（ハッシュ）
    ///
    /// - Gemini:
    ///   - `projectHash`（OSS 実装 `sha256(project_root).hex`）
    /// - Claude:
    ///   - 通常は `cwd` を project_root とみなし `sha256(cwd)` を計算することを推奨。
    /// - Codex:
    ///   - `payload.cwd` を project_root とみなし同様に計算することを推奨。
    ///
    /// ただし、ベンダーごとに「プロジェクトルート判定ロジック」が異なるため、
    /// 完全一致を必ずしも保証しない。あくまで「プロジェクトを識別するキー」として扱う。
    pub project_hash: String,

    /// プロジェクトのルートパス（絶対パス想定）。
    ///
    /// - Claude: `cwd`
    /// - Codex : `payload.cwd`
    /// - Gemini: ログからは復元不能なため `None`
    pub project_root: Option<String>,

    /// セッション単位の ID。
    ///
    /// - Claude: `sessionId`
    /// - Codex :
    ///     - `payload.id` が存在する場合はそれを優先。
    ///     - なければ「rollout ファイル名」や、ファイルパスから合成。
    /// - Gemini:
    ///     - `sessionId`
    pub session_id: Option<String>,

    /// イベント固有の ID（セッション内で一意であることが望ましい）。
    ///
    /// - Claude:
    ///     - user_message: `uuid` や `messageId` など
    ///     - assistant / tool系: `message.id` / `tool_use.id` などから合成
    /// - Codex:
    ///     - tool 系: `payload.call_id`
    ///     - その他: 必要に応じて `timestamp + シーケンス` 等で合成
    /// - Gemini:
    ///     - messages[]: `messages[].id`
    ///     - CLIログ: `messageId` を文字列化
    pub event_id: Option<String>,

    /// 「どの user_message に属するイベントか」を示す ID。
    ///
    /// ルール:
    /// - `event_type == EventType::UserMessage` のイベントだけ `parent_event_id = None`。
    /// - それ以外のイベント（assistant_message / reasoning / tool_call / tool_result /
    ///   file_snapshot / meta / log）は、同一 `session_id` 内で直近の user_message の
    ///   `event_id` を `parent_event_id` に設定する。
    ///
    /// これにより、「1ユーザ発話をルートとする会話ターン」が復元できる。
    ///
    /// 注意:
    /// - tool_call / tool_result の親は assistant_message ではなく「ユーザーメッセージのルート」とする。
    /// - tool_call と tool_result の対応は `tool_call_id` で表現する。
    pub parent_event_id: Option<String>,

    /// イベント発生時刻（RFC3339, UTC）
    ///
    /// - 可能な限りベンダーの元ログの timestamp を使用する。
    pub ts: String,

    // -------------------------------------------------------------------------
    // イベントの性質
    // -------------------------------------------------------------------------

    pub event_type: EventType,
    pub role: Option<Role>,
    pub channel: Option<Channel>,

    /// 人間向けのテキスト（本文 or 要約）
    ///
    /// - user_message: ユーザー発話
    /// - assistant_message: モデル応答の本文
    /// - reasoning: thinking / thoughts / reasoning の本文
    /// - tool_call: 引数のサマリ
    /// - tool_result: 出力のサマリ
    /// - file_snapshot: 対象ファイル数などの短い要約
    pub text: Option<String>,

    // -------------------------------------------------------------------------
    // ツール / コマンド実行
    // -------------------------------------------------------------------------

    /// 呼び出されたツール名 / コマンド名。
    ///
    /// - Claude: `message.content[].name`（`type == "tool_use"`）
    /// - Codex : `payload.name`（例: "shell", "apply_patch"）
    /// - Gemini: `toolCalls[].name`
    pub tool_name: Option<String>,

    /// ツール呼び出しの ID。
    ///
    /// - Claude: `message.content[].id` または `tool_use_id`
    /// - Codex : `payload.call_id`
    /// - Gemini: `toolCalls[].id`
    ///
    /// tool_call イベントと tool_result イベントは、この `tool_call_id` によって join できる。
    pub tool_call_id: Option<String>,

    /// ツール実行ステータス。
    ///
    /// - Claude:
    ///     - `toolUseResult.status == "completed"` → Success
    ///     - `interrupted == true` など → Error
    /// - Codex:
    ///     - `status == "completed"` → Success
    ///     - その他 → Error または Unknown
    /// - Gemini:
    ///     - `toolCalls[].status` の値によってマッピング
    pub tool_status: Option<ToolStatus>,

    /// ツール実行のレイテンシ (ms)。vendorログから取れれば埋める。
    pub tool_latency_ms: Option<u64>,

    /// ツール終了コード（exit code）。
    pub tool_exit_code: Option<i32>,

    // -------------------------------------------------------------------------
    // ファイル / コード
    // -------------------------------------------------------------------------

    /// 主に対象となったファイルパス。
    ///
    /// - Claude: `toolUseResult.filePath` / `toolUseResult.file.filePath` 等
    /// - Codex : apply_patch が対象とするファイル等（実装側で抽出）
    /// - Gemini: ファイルツールを使う場合に設定（なければ None）
    pub file_path: Option<String>,

    /// 主な言語（例: "rust", "typescript"）を推定できる場合に設定。
    pub file_language: Option<String>,

    /// ファイル操作種別（任意）。
    ///
    /// 実装側で必要になった時点で利用すればよい。
    pub file_op: Option<String>,

    // -------------------------------------------------------------------------
    // モデル / トークン
    // -------------------------------------------------------------------------

    /// 使用モデル名。
    ///
    /// - Claude: `message.model`
    /// - Codex : `payload.model`
    /// - Gemini: `messages[].model`
    pub model: Option<String>,

    /// イベント単位の推論トークン数（可能な限り「そのイベントで消費された分」）。
    pub tokens_input: Option<u64>,
    pub tokens_output: Option<u64>,
    pub tokens_total: Option<u64>,
    pub tokens_cached: Option<u64>,
    pub tokens_thinking: Option<u64>,
    pub tokens_tool: Option<u64>,

    /// ベンダー側エージェント ID（Claude の `agentId` 等）。
    pub agent_id: Option<String>,

    // -------------------------------------------------------------------------
    // ベンダー固有情報
    // -------------------------------------------------------------------------

    /// ベンダー固有情報（元レコード or 軽量サマリ）。
    ///
    /// 現段階では JSON をそのまま格納してもよいが、
    /// 必要に応じてサマライズ・トリミングする実装ポリシーを導入する。
    pub raw: serde_json::Value,
}

impl AgentEventV1 {
    pub const SCHEMA_VERSION: &'static str = "agtrace.event.v1";

    /// 便利なコンストラクタ（最低限の必須フィールドだけを受け取る）。
    pub fn new(
        source: Source,
        project_hash: String,
        ts: String,
        event_type: EventType,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            source,
            project_hash,
            project_root: None,
            session_id: None,
            event_id: None,
            parent_event_id: None,
            ts,

            event_type,
            role: None,
            channel: None,
            text: None,

            tool_name: None,
            tool_call_id: None,
            tool_status: None,
            tool_latency_ms: None,
            tool_exit_code: None,

            file_path: None,
            file_language: None,
            file_op: None,

            model: None,
            tokens_input: None,
            tokens_output: None,
            tokens_total: None,
            tokens_cached: None,
            tokens_thinking: None,
            tokens_tool: None,

            agent_id: None,
            raw: serde_json::Value::Null,
        }
    }
}
```

この Rust コードが、そのまま「スキーマ＋マッピング仕様」を表現しています。

* 型レベルで **何を保存するか** を固定
* doc コメントで **Claude / Codex / Gemini のどのフィールドをどこに入れるか** を明文化

このファイルを `src/model/agent_event.rs` などとしてレポジトリに置き、
各ベンダー用の正規化ロジックはこの型に合わせて実装していけば、
「仕様を Rust で確定した」状態になります。


---

### `src/normalize_codex.rs` の例

```rust
//! Codex rollout logs → AgentEventV1 正規化
//!
//! 想定する生ログ構造（JSONL 1 行ごと）:
//! {
//!   "timestamp": "2025-11-03T01:49:22.517Z",
//!   "type": "response_item" | "event_msg" | "turn_context" ...,
//!   "payload": {
//!     "type": "message" | "reasoning" | "token_count" | ...,
//!     "role": "user" | "assistant" | ...,
//!     "content": [...],
//!     "name": "shell" | "apply_patch" | ...,
//!     "call_id": "call_xxx",
//!     "status": "completed" | "failed" | ...,
//!     "cwd": "/path/to/project",
//!     "model": "gpt-5-codex",
//!     "info": { "last_token_usage": { ... } },
//!     ...
//!   }
//! }

use std::path::Path;

use serde_json::Value;
use sha2::{Digest, Sha256};

// AgentEventV1 スキーマを定義したモジュールをインポート
// パスはプロジェクト構成に合わせて調整してください。
use crate::model::{
    AgentEventV1, Channel, EventType, Role, Source, ToolStatus,
};

/// project_root 文字列から project_hash を計算するヘルパ。
///
/// Gemini CLI の getProjectHash と同等:
/// sha256(projectRoot).hex
fn project_hash_from_root(project_root: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(project_root.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Codex の JSON/JSONL ファイル 1 本を読み込み、
/// 正規化済み AgentEventV1 の Vec を返す。
///
/// - `session_id`: この rollout ファイル全体で共通のセッション ID として使う。
///   - payload.id がある場合は、そちらを優先してもよい（後述の normalize_stream 内）
/// - `project_root`: payload.cwd が取れない場合のフォールバックとして使える。
pub fn normalize_codex_file(
    path: &Path,
    session_id: &str,
    project_root: Option<&str>,
) -> anyhow::Result<Vec<AgentEventV1>> {
    let text = std::fs::read_to_string(path)?;
    let mut records: Vec<Value> = Vec::new();

    // JSONL として読む
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line)?;
        records.push(v);
    }

    Ok(normalize_codex_stream(
        records.into_iter(),
        session_id,
        project_root,
    ))
}

/// Codex のレコードストリームを AgentEventV1 に正規化する。
///
/// - `records`: Codex rollout の JSON レコード (serde_json::Value)
/// - `session_id`: このストリーム全体で共通のセッション ID
/// - `project_root`: 既知であれば project_root として使う。なければ payload.cwd から決定する。
pub fn normalize_codex_stream<I>(
    records: I,
    session_id: &str,
    project_root: Option<&str>,
) -> Vec<AgentEventV1>
where
    I: IntoIterator<Item = Value>,
{
    let mut events = Vec::new();

    // セッション内で直近の user_message の event_id を保持
    let mut last_user_event_id: Option<String> = None;

    // シーケンス番号（event_id 合成用）
    let mut seq: u64 = 0;

    // project_root / project_hash を一旦決めておく
    let mut project_root_str: Option<String> = project_root.map(|s| s.to_string());
    let mut project_hash: Option<String> = project_root_str
        .as_deref()
        .map(project_hash_from_root);

    for rec in records {
        seq += 1;

        let ts = rec
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let payload = rec.get("payload").cloned().unwrap_or(Value::Null);
        let payload_obj = payload.as_object();

        // payload.cwd から project_root を補正
        if project_root_str.is_none() {
            if let Some(cwd) = payload_obj
                .and_then(|m| m.get("cwd"))
                .and_then(|v| v.as_str())
            {
                project_root_str = Some(cwd.to_string());
                project_hash = Some(project_hash_from_root(cwd));
            }
        }

        // project_hash がまだ無ければ、最後の手段として "unknown" を入れておく
        let project_hash_val = project_hash
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        // ベースイベントを構築
        let mut ev = AgentEventV1::new(
            Source::Codex,
            project_hash_val,
            ts.clone(),
            EventType::Meta, // とりあえず Meta で初期化し、後で上書き
        );

        ev.session_id = Some(session_id.to_string());
        ev.project_root = project_root_str.clone();

        // -------------------------
        // payload からフィールド抽出
        // -------------------------
        let p_type = payload_obj
            .and_then(|m| m.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let p_role = payload_obj
            .and_then(|m| m.get("role"))
            .and_then(|v| v.as_str());

        let p_name = payload_obj
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str());

        let p_status = payload_obj
            .and_then(|m| m.get("status"))
            .and_then(|v| v.as_str());

        let p_model = payload_obj
            .and_then(|m| m.get("model"))
            .and_then(|v| v.as_str());

        if let Some(model) = p_model {
            ev.model = Some(model.to_string());
        }

        // トークン情報 (last_token_usage) を単イベント usage とみなす
        if let Some(info) = payload_obj.and_then(|m| m.get("info")).and_then(|v| v.as_object()) {
            if let Some(last) = info
                .get("last_token_usage")
                .and_then(|v| v.as_object())
            {
                ev.tokens_input = last
                    .get("input_tokens")
                    .and_then(|v| v.as_u64());
                ev.tokens_output = last
                    .get("output_tokens")
                    .and_then(|v| v.as_u64());
                ev.tokens_total = last
                    .get("total_tokens")
                    .and_then(|v| v.as_u64());
                ev.tokens_cached = last
                    .get("cached_input_tokens")
                    .and_then(|v| v.as_u64());
                ev.tokens_thinking = last
                    .get("reasoning_output_tokens")
                    .and_then(|v| v.as_u64());
            }
        }

        // tool_call_id
        if let Some(call_id) = payload_obj
            .and_then(|m| m.get("call_id"))
            .and_then(|v| v.as_str())
        {
            ev.tool_call_id = Some(call_id.to_string());
        }

        // event_id を決める（call_id があればそれを優先）
        ev.event_id = if let Some(ref cid) = ev.tool_call_id {
            Some(cid.clone())
        } else {
            // セッション内一意であればよいので、timestamp + seq で合成
            Some(format!("{}#{}", ev.ts, seq))
        };

        // parent_event_id: 直近の user_message を参照（user_message 自身は None）
        ev.parent_event_id = last_user_event_id.clone();

        // -------------------------
        // event_type / role / channel / text の決定
        // -------------------------

        // 1) message (user / assistant)
        if p_type == "message" && p_role.is_some() {
            match p_role.unwrap() {
                "user" => {
                    ev.event_type = EventType::UserMessage;
                    ev.role = Some(Role::User);
                    ev.channel = Some(Channel::Chat);

                    // parent_event_id は user_message なので None にリセット
                    ev.parent_event_id = None;
                    last_user_event_id = ev.event_id.clone();

                    // text: content[].text or payload.text
                    ev.text = extract_codex_message_text(&payload);
                }
                "assistant" => {
                    ev.event_type = EventType::AssistantMessage;
                    ev.role = Some(Role::Assistant);
                    ev.channel = Some(Channel::Chat);
                    ev.text = extract_codex_message_text(&payload);
                    // parent_event_id は last_user_event_id のまま
                }
                _ => {
                    // その他の role は SystemMessage として扱うこともできる
                    ev.event_type = EventType::SystemMessage;
                    ev.role = Some(Role::System);
                    ev.channel = Some(Channel::System);
                    ev.text = extract_codex_message_text(&payload);
                }
            }
        }
        // 2) reasoning
        else if p_type == "reasoning" {
            ev.event_type = EventType::Reasoning;
            ev.role = Some(Role::Assistant);
            ev.channel = Some(Channel::Chat);
            // reasoning テキストは payload.text に入っているケースが多い
            ev.text = payload_obj
                .and_then(|m| m.get("text"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
        // 3) tool_call
        else if p_name.is_some() {
            ev.event_type = EventType::ToolCall;
            ev.role = Some(Role::Assistant);
            ev.channel = match p_name.unwrap() {
                "shell" => Some(Channel::Terminal),
                "apply_patch" => Some(Channel::Editor),
                _ => Some(Channel::Chat),
            };
            ev.tool_name = Some(p_name.unwrap().to_string());
            // 引数をそのまま text として雑に入れる（必要に応じてパース）
            ev.text = payload_obj
                .and_then(|m| m.get("arguments"))
                .and_then(|v| v.as_str())
                .map(|s| truncate(s, 2000));
        }
        // 4) tool_result
        else if p_status.is_some() {
            ev.event_type = EventType::ToolResult;
            ev.role = Some(Role::Assistant); // or Tool
            ev.channel = Some(Channel::Terminal); // とりあえず shell 前提。細かく分類したければ name を見る必要あり。

            ev.tool_status = Some(match p_status.unwrap() {
                "completed" => ToolStatus::Success,
                "failed" | "error" => ToolStatus::Error,
                _ => ToolStatus::Unknown,
            });

            // output（JSON string の中の "output" フィールド）をサマリとして text に入れる
            ev.text = payload_obj
                .and_then(|m| m.get("output"))
                .and_then(|v| v.as_str())
                .map(|s| truncate(s, 2000));
        }
        // 5) その他: meta 系
        else {
            ev.event_type = EventType::Meta;
            ev.role = Some(Role::System);
            ev.channel = Some(Channel::System);
            ev.text = payload_obj
                .and_then(|m| m.get("text"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }

        // raw に元レコードを置いておく（必要に応じてサマライズしてもよい）
        ev.raw = rec;

        events.push(ev);
    }

    events
}

/// Codex の message payload からユーザー/アシスタントの本文を抽出するヘルパ。
///
/// - payload.content[].text (input_text / output_text) を優先
/// - なければ payload.text
fn extract_codex_message_text(payload: &Value) -> Option<String> {
    let obj = payload.as_object()?;
    if let Some(content) = obj.get("content") {
        if let Some(arr) = content.as_array() {
            let mut texts = Vec::new();
            for c in arr {
                if let Some(cobj) = c.as_object() {
                    let c_type = cobj
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if c_type == "input_text" || c_type == "output_text" {
                        if let Some(t) = cobj.get("text").and_then(|v| v.as_str()) {
                            texts.push(t.to_string());
                        }
                    }
                }
            }
            if !texts.is_empty() {
                return Some(truncate(&texts.join("\n"), 2000));
            }
        }
    }
    // fallback: payload.text
    obj.get("text")
        .and_then(|v| v.as_str())
        .map(|s| truncate(s, 2000))
}

/// 長すぎる文字列を N 文字で切り詰める簡易関数。
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "...(truncated)"
    }
}
```

---

この実装でやっていること（仕様との対応）:

* `Source::Codex` / `EventType` / `Role` / `Channel` / `ToolStatus` を仕様どおりに付与
* `project_hash` は `project_root` / `payload.cwd` から `sha256` で算出（なければ "unknown"）
* `session_id` は呼び出し側から与える（rollout ファイル単位）
* `event_id`:

  * `tool_call_id` があればそれを優先
  * なければ `timestamp#seq` の合成
* `parent_event_id`:

  * `UserMessage` のときのみ `None` にし、`last_user_event_id` を更新
  * それ以外のイベントは常に「直近の user_message」の ID を持つ
* `event_type` の判定:

  * `p_type == "message" && role == "user"` → `UserMessage`
  * `p_type == "message" && role == "assistant"` → `AssistantMessage`
  * `p_type == "reasoning"` → `Reasoning`
  * `name` がある → `ToolCall`
  * `status` がある → `ToolResult`
  * それ以外 → `Meta`
* `text`:

  * message: `content[].text` or `payload.text`
  * tool_call: `arguments` 文字列（トリム付き）
  * tool_result: `output` 文字列（トリム付き）

これで **Codex → AgentEventV1** のマッピング仕様を「動くコード」の形で一段階前に進めました。

次のステップとしては、これと同じパターンで:

* Claude Code の `normalize_claude_stream`
* Gemini CLI の `normalize_gemini_stream`

を実装していくことになりますが、まずは Codex でイベントの形・粒度・parent の付き方を確認してから、Claude/Gemini を合わせに行くのが安全な進め方だと思います。
