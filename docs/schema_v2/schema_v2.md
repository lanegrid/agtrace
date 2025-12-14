### Rust Schema Definition v2

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// エージェントイベント全体を表す構造体
/// DBのテーブル行と1対1で対応することを想定しています。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    /// イベントの一意なID
    pub id: Uuid,

    /// セッション/トレースID (一連の会話全体を束ねるID)
    pub trace_id: Uuid,

    /// 時系列上の親イベントID (Linked List構造)
    /// Rootイベント(最初のUser入力)の場合は None
    pub parent_id: Option<Uuid>,

    /// イベント発生時刻 (UTC)
    pub timestamp: DateTime<Utc>,

    /// イベントの種類と内容 (Enum)
    #[serde(flatten)]
    pub payload: EventPayload,

    /// プロバイダー固有の生データや、デバッグ情報を格納するメタデータ
    /// 例: Codexの "call_id" や Geminiの "finish_reason" など
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// イベントの種類ごとのペイロード定義
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
#[serde(rename_all = "snake_case")]
pub enum EventPayload {
    /// 1. ユーザー入力 (Trigger)
    User(UserPayload),

    /// 2. アシスタントの思考プロセス (Geminiのthoughts等)
    Reasoning(ReasoningPayload),

    /// 3. ツール実行要求 (Action Request)
    /// ※ ここに TokenUsage がサイドカーとしてぶら下がります
    ToolCall(ToolCallPayload),

    /// 4. ツール実行結果 (Action Result)
    ToolResult(ToolResultPayload),

    /// 5. アシスタントのテキスト回答 (Final Response)
    /// ※ ここに TokenUsage がサイドカーとしてぶら下がります
    Message(MessagePayload),

    /// 6. コスト情報 (Sidecar / Leaf Node)
    /// コンテキストには含めず、コスト計算のために使用します
    TokenUsage(TokenUsagePayload),
}

// --- 詳細ペイロード定義 ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPayload {
    /// ユーザーの入力テキスト
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPayload {
    /// 思考内容
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallPayload {
    /// 実行するツール名
    pub name: String,
    /// ツールへの引数 (JSON)
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultPayload {
    /// ツールの実行結果 (テキスト、JSON文字列、エラーメッセージ等)
    pub output: String,

    /// 論理的な親 (Tool Call) への参照ID
    /// parent_id (時系列上の親) とは別に、どのコールの結果かを明示します
    pub tool_call_id: Uuid,

    /// 実行成功か失敗か
    #[serde(default)]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    /// 回答テキスト
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsagePayload {
    /// 入力トークン数 (増分)
    pub input_tokens: i32,
    /// 出力トークン数 (増分)
    pub output_tokens: i32,
    /// 合計トークン数 (増分)
    pub total_tokens: i32,

    /// 詳細内訳 (Optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<TokenUsageDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageDetails {
    /// キャッシュが効いた入力トークン数 (Claude等)
    pub cache_read_input_tokens: Option<i32>,
    /// 推論トークン数 (o1/Gemini等)
    pub reasoning_output_tokens: Option<i32>,
}

// --- ヘルパーメソッド ---

impl AgentEvent {
    /// このイベントが「生成コスト」を持つ主体かどうか (TokenUsageの親になりうるか)
    pub fn is_generation_event(&self) -> bool {
        matches!(
            self.payload,
            EventPayload::ToolCall(_) | EventPayload::Message(_)
        )
    }

    /// このイベントが LLM のコンテキスト履歴に含まれるべきかどうか
    /// (TokenUsage は履歴には含めない)
    pub fn is_context_event(&self) -> bool {
        !matches!(self.payload, EventPayload::TokenUsage(_))
    }
}
```

-----

### 設計のポイント解説

1.  **`parent_id` (時系列) と `tool_call_id` (論理) の分離**:

      * `AgentEvent.parent_id`: `Uuid` 型。これは時系列チェーン（LinkedList）を構築するためのポインタです。
      * `ToolResultPayload.tool_call_id`: `Uuid` 型。これは「この結果はどのリクエストに対するものか」をプログラム的に特定するための論理参照です。

2.  **`TokenUsage` の扱い (Sidecarパターン)**:

      * `EventPayload::TokenUsage` として独立させています。
      * これを保存する際は、対応する `ToolCall` または `Message` イベントの ID を `parent_id` にセットします。
      * ヘルパーメソッド `is_context_event()` を用意し、LLM に履歴を投げる際はこれをフィルタリング（除外）できるようにしています。

3.  **JSON引数の扱い**:

      * `ToolCallPayload.arguments` は `serde_json::Value` にしました。
      * これはプロバイダーによって JSON オブジェクトで来たり、JSON 文字列で来たりする揺れを吸収しやすくするためです（Ingestor側でパースして `Value` に統一して保存することを推奨します）。

### 使用例 (Mock)

これまでの議論にあった「Reasoning -\> ToolCall -\> Token -\> Result」の流れを作る例です。

```rust
fn create_mock_chain() {
    let trace_id = Uuid::new_v4();
    let timestamp = Utc::now();

    // 1. User
    let user_event = AgentEvent {
        id: Uuid::new_v4(),
        trace_id,
        parent_id: None,
        timestamp,
        payload: EventPayload::User(UserPayload {
            text: "Calculate 1+1".to_string(),
        }),
        metadata: None,
    };

    // 2. Reasoning (Parent: User)
    let reasoning_event = AgentEvent {
        id: Uuid::new_v4(),
        trace_id,
        parent_id: Some(user_event.id),
        timestamp,
        payload: EventPayload::Reasoning(ReasoningPayload {
            text: "I should use python.".to_string(),
        }),
        metadata: None,
    };

    // 3. Tool Call (Parent: Reasoning)
    let tool_call_event = AgentEvent {
        id: Uuid::new_v4(),
        trace_id,
        parent_id: Some(reasoning_event.id),
        timestamp,
        payload: EventPayload::ToolCall(ToolCallPayload {
            name: "python".to_string(),
            arguments: serde_json::json!({"code": "print(1+1)"}),
        }),
        metadata: None,
    };

    // 4. Token Usage (Parent: Tool Call) <-- Sidecar / Leaf Node
    let token_event = AgentEvent {
        id: Uuid::new_v4(),
        trace_id,
        parent_id: Some(tool_call_event.id), // 親は Tool Call
        timestamp,
        payload: EventPayload::TokenUsage(TokenUsagePayload {
            input_tokens: 100,
            output_tokens: 20,
            total_tokens: 120,
            details: None,
        }),
        metadata: None,
    };

    // 5. Tool Result (Parent: Tool Call) <-- Context Chain continues
    // ※ Token Event は無視して、直前のコンテキスト上の親(Tool Call)の後につなぐ
    //   (あるいは実装によっては Token Event を parent_id にしても良いが、
    //    is_context_event() でフィルタすれば問題ない)
    let tool_result_event = AgentEvent {
        id: Uuid::new_v4(),
        trace_id,
        parent_id: Some(tool_call_event.id), // Tool Callの続き
        timestamp,
        payload: EventPayload::ToolResult(ToolResultPayload {
            output: "2".to_string(),
            tool_call_id: tool_call_event.id, // 論理参照
            is_error: false,
        }),
        metadata: None,
    };
}
```
