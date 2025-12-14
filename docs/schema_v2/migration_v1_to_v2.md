既存のコードベース（約2,000行）はまだ手頃なサイズですが、コアのデータ構造を変更するため、**「並行稼働（Parallel Adoption）」** 戦略をとるのが最も安全で確実です。

一気に `V1` を消して書き換えると、コンパイルエラーの修正に追われ、ロジックの正当性を検証できなくなります。

以下に、**既存の機能を壊さずに、段階的に新スキーマ（V2）へ移行する5つのステップ**を提案します。

-----

### 全体戦略：Strangler Fig パターン（絞め殺し植物パターン）

既存の `AgentEventV1` の横に新しい `AgentEvent` を作り、データの入り口（Ingest）と出口（Engine）を徐々に新しいパイプラインに切り替えていきます。

-----

## Phase 1: 新旧共存 (Types Layer)

まず、`agtrace-types` に新しいスキーマを定義しますが、古い定義は消しません。

**アクション:**

1.  `crates/agtrace-types/src/v2.rs` を作成し、前回確定した新スキーマ（`AgentEvent`, `EventPayload`など）を実装する。
2.  `crates/agtrace-types/src/lib.rs` で `v2` モジュールを公開する。

**目的:**
コンパイルを通したまま、新しいデータ構造を使える状態にします。

```rust
// crates/agtrace-types/src/lib.rs

// 既存 (V1) - そのまま維持
pub use util::*;
mod v1; // 既存の AgentEventV1 定義をここへ移動、またはそのまま
pub use v1::*;

// 新規 (V2) - 追加
pub mod v2;
```

-----

## Phase 2: 正規化層の実装 (Ingestion Layer)

ここが最大の難関であり、今回の移行の肝です。
現在 `agtrace-providers` のテストコード内や `engine` 内で場当たり的に行われている「データの解釈」を、**「正規化（Normalization）」** という明確な責務として切り出します。

**アクション:**

1.  新しいクレート `agtrace-ingest` (または `agtrace-core`) を作成するか、`agtrace-providers` 内に変換ロジックを実装します。
2.  **Converter (V1 -\> V2) ではなく、Provider Raw Data -\> V2 の変換** を実装します。
      * *理由:* `AgentEventV1` は情報が欠落している（Fat Structの弊害）ため、そこから `V2` を作るのは困難です。生データ（JSON）から直接 `V2` を生成する方が正確です。

**実装イメージ (Converter):**

```rust
struct EventBuilder {
    trace_id: Uuid,
    parent_id: Option<Uuid>,
    // ツール呼び出しIDとUUIDの対応表を一時保持
    tool_map: HashMap<String, Uuid>,
}

impl EventBuilder {
    // Gemini等の「1レコード複数イベント」をここで展開(Unfold)する
    fn ingest_gemini(&mut self, raw: &GeminiRawEvent) -> Vec<agtrace_types::v2::AgentEvent> {
        let mut events = Vec::new();

        // 1. Reasoning
        if let Some(thought) = raw.thoughts {
             let id = Uuid::new_v4();
             // ... Reasoningイベント作成 ...
             self.parent_id = Some(id); // 親IDを更新
        }

        // 2. ToolCall
        if let Some(call) = raw.tool_calls {
             let id = Uuid::new_v4();
             // マップに登録 (プロバイダーID -> UUID)
             self.tool_map.insert(call.id, id);
             // ... ToolCallイベント作成 ...
             self.parent_id = Some(id);
        }

        // ... Message, TokenUsage ...

        events
    }
}
```

-----

## Phase 3: エンジンの並行実装 (Engine Layer)

`agtrace-engine` 内で、V2データを使う新しいロジックを実装します。既存の `span.rs` 等は触らず、`span_v2.rs` を作ります。

**アクション:**

1.  `crates/agtrace-engine/src/span_v2.rs` を作成。
2.  `build_spans_v2(events: &[v2::AgentEvent]) -> Vec<Span>` を実装。

**ここでの変化:**
これまでの「推測ロジック（次がToolResultか？）」が、「ID照合ロジック」に変わります。

```rust
// span_v2.rs のイメージ
// 以前のような「保留バッファ(pending_tools)」の複雑な管理が消え、
// 単純な HashMap<Uuid, ToolAction> へのマッピングになります。

pub fn build_spans_v2(events: &[v2::AgentEvent]) -> Vec<Span> {
    let mut spans = Vec::new();
    // ...
    for event in events {
        match &event.payload {
            EventPayload::ToolCall(call) => {
                // event.id をキーにして Span内のツールリストに登録
            },
            EventPayload::ToolResult(res) => {
                // res.tool_call_id (UUID) を使って
                // 登録済みのToolActionをO(1)で引いて更新するだけ
                // 順序が前後しても、非同期で遅れても関係なく特定可能
            }
            // ...
        }
    }
    spans
}
```

-----

## Phase 4: 検証とスイッチ (Validation)

新旧の実装が出力する `SessionSummary` や `AnalysisReport` を比較します。

**アクション:**

1.  テストケース（`provider_snapshots`）で、同じ入力データから `V1パイプライン` と `V2パイプライン` 両方を実行する。
2.  結果（Spanの数、トークン計算結果など）が一致、あるいはV2の方がより正確であることを確認する。
3.  問題なければ、CLIやAPIのエントリポイントを `V2` 版の関数に切り替える。

-----

## Phase 5: 負債の削除 (Cleanup)

V1パイプラインが使われていないことを確認したら、古いコードを削除します。

**アクション:**

1.  `agtrace-engine/src/span.rs`, `turn.rs` などを削除し、`_v2` をリネームして正にする。
2.  `agtrace-types/src/v1.rs` (旧 `AgentEventV1`) を削除。
3.  `agtrace-engine` 内の不要な推測ロジック（`extract_input_summary` でのJSONパースなど）が残っていれば削除。

-----

## 具体的なネクストステップ

まずは **Phase 1 & 2** に着手するのが良いでしょう。
以下の順序で作業を進めることを推奨します。

1.  **`agtrace-types` の更新:**
    `v2.rs` を作り、提案した `AgentEvent`, `EventPayload` 等の定義をコピペして配置する。
2.  **実データの変換テスト:**
    `agtrace-providers` のテストコード等で、既存の JSON スナップショット（Gemini, Codex, Claude）を読み込み、**「コンパイルエラーなく V2 の `Vec<AgentEvent>` に変換できるか？」** を検証するコンバータ関数を書く。

この「変換テスト」が通れば、スキーマ設計は成功です。そこから先（Engineの実装）は、単純作業になります。
