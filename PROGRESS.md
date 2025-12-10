# Provider Schema Refactoring Progress

## 目的

プロバイダのログパース処理を`serde_json::Value`を使った動的パースから、型安全な構造体を使ったパースに移行する。

## 背景

### 現在の問題点

1. **型安全性の欠如**
   - 全てのプロバイダで`serde_json::Value`を使った動的パース
   - フィールドアクセスが`.get("field").and_then(|v| v.as_str())`のパターンで繰り返される
   - パース時のエラーが実行時にしか分からない

2. **コードの複雑性**
   - mapper.rsが長く複雑（Claude: 582行、Codex: 240行、Gemini: 178行）
   - データ構造が暗黙的で、ドキュメントやサンプルデータを見ないと理解できない
   - 同じようなパターン（フィールド抽出）が繰り返される

3. **保守性の低さ**
   - プロバイダのログ形式が変更されたときに影響範囲が分かりにくい
   - 新しいフィールドを追加する際、複数箇所を修正する必要がある

## アプローチ

各プロバイダごとに**スキーマ型（構造体）を定義**し、serdeでパースする。

```
src/providers/<provider>/
  ├── mod.rs          # Provider実装
  ├── schema.rs       # 新規: プロバイダ固有の型定義
  ├── io.rs           # スキーマ型を使ったパース
  └── mapper.rs       # スキーマ型からAgentEventV1への変換
```

### メリット

1. **型安全性**: コンパイル時にフィールドの存在をチェック
2. **可読性**: 構造体定義でデータ構造が明確になる
3. **保守性**: フィールド追加・変更時の影響範囲が明確
4. **パフォーマンス**: 不要な動的チェックが減る

## 進捗状況

### ✅ 完了

#### 1. Geminiプロバイダ（完全実装）

- [x] `src/providers/gemini/schema.rs` 作成
  - `GeminiSession`, `GeminiMessage` (enum), `UserMessage`, `GeminiAssistantMessage`, `InfoMessage`
  - `Thought`, `ToolCall`, `TokenUsage` など
- [x] `src/providers/gemini/io.rs` 更新
  - `normalize_gemini_file()`: `GeminiSession`を直接デシリアライズ
  - `extract_gemini_header()`: スキーマ型を使用
  - `extract_project_hash_from_gemini_file()`: スキーマ型を使用
- [x] `src/providers/gemini/mapper.rs` 更新
  - `normalize_gemini_session(&GeminiSession)`: パターンマッチで型安全に処理
  - 178行 → コードがシンプルで読みやすく
- [x] スナップショットテスト作成・成功
  - `tests/provider_snapshots.rs::test_gemini_parse_snapshot`

**コミット**:
- `feat: add typed schema for Gemini provider with snapshot tests` (6e48ad6)
- `test: add snapshot tests for Codex and Claude providers` (891bbf2)

#### 2. テストインフラ

- [x] `insta` クレート追加 (Cargo.toml)
- [x] 全プロバイダのスナップショットテスト作成
  - `tests/provider_snapshots.rs`
  - Gemini, Codex, Claude 各3イベントのスナップショット
- [x] 全テスト成功確認

#### 3. Codexプロバイダ（スキーマ定義のみ）

- [x] `src/providers/codex/schema.rs` 作成
  - `CodexRecord` (enum): `SessionMeta`, `ResponseItem`, `EventMsg`, `TurnContext`
  - `SessionMetaPayload`, `ResponseItemPayload` (enum), `EventMsgPayload` (enum)
  - `MessagePayload`, `MessageContent` (enum), `ReasoningPayload`
  - `FunctionCallPayload`, `CustomToolCallPayload`, `TokenInfo`, `TokenUsage` など

### 🚧 未完了

#### 4. Codexプロバイダ（io.rs / mapper.rs更新）

**課題**: io.rsとmapper.rsを同時に更新する必要がある

- [ ] `src/providers/codex/io.rs` 更新
  - `normalize_codex_file()`: `Vec<CodexRecord>`を使う
  - `extract_codex_header()`: パターンマッチでスキーマ型を使う
  - `extract_cwd_from_codex_file()`: スキーマ型を使う
- [ ] `src/providers/codex/mapper.rs` 更新（240行）
  - `normalize_codex_stream()`: `Vec<CodexRecord>`を受け取る
  - `CodexRecord`のenumパターンマッチで各レコードタイプを処理
  - 複雑な条件分岐を型安全に書き換え

**実装メモ**:
```rust
// Before (Value)
let p_type = payload_obj
    .and_then(|m| m.get("type"))
    .and_then(|v| v.as_str())
    .unwrap_or("");

// After (CodexRecord)
match record {
    CodexRecord::ResponseItem(response) => {
        match &response.payload {
            ResponseItemPayload::Message(msg) => {
                // 型安全にアクセス
            }
            ResponseItemPayload::FunctionCall(call) => { ... }
            _ => {}
        }
    }
    CodexRecord::EventMsg(event) => { ... }
    _ => {}
}
```

#### 5. Claudeプロバイダ（未着手）

- [ ] `src/providers/claude/schema.rs` 作成
  - Claudeのログ形式は最も複雑（582行のmapper.rs）
  - JSONL形式、各行が異なる`type`フィールドを持つ
  - `file-history-snapshot`, `user`, `assistant` など多様なタイプ
- [ ] `src/providers/claude/io.rs` 更新
- [ ] `src/providers/claude/mapper.rs` 更新

**データ構造の例** (サンプルから):
```json
{"type":"file-history-snapshot", "messageId":"...", "snapshot":{...}}
{"parentUuid":null, "isSidechain":false, "type":"user", "message":{...}}
{"type":"assistant", "message":{"model":"...", "content":[{"type":"thinking",...}]}}
```

**推奨アプローチ**:
```rust
#[derive(Deserialize)]
#[serde(tag = "type")]
enum ClaudeRecord {
    #[serde(rename = "file-history-snapshot")]
    FileHistorySnapshot { ... },
    #[serde(rename = "user")]
    User { ... },
    #[serde(rename = "assistant")]
    Assistant { ... },
    ...
}
```

## 今後の作業手順

### ステップ1: Codex実装完了

1. `io.rs`と`mapper.rs`を同時に更新
   - io.rsで`Vec<CodexRecord>`を返すように変更
   - mapper.rsでパターンマッチを使った処理に書き換え
2. ビルドエラーを解消しながら段階的に進める
3. スナップショットテストで動作確認
4. コミット: `feat: update Codex provider to use typed schema`

### ステップ2: Claude実装

1. サンプルデータを分析してスキーマ設計
   - `samples-tmp/.claude/projects/` のJSONLを確認
   - 各`type`のバリエーションを洗い出し
2. `schema.rs`作成
3. `io.rs`更新
4. `mapper.rs`更新（最も複雑、段階的に）
5. スナップショットテストで確認
6. コミット: `feat: update Claude provider to use typed schema`

### ステップ3: 最終検証

1. 全テスト実行: `cargo test`
2. 実際のデータでscan/list/show動作確認
3. パフォーマンス比較（オプション）
4. ドキュメント更新

## 技術的な注意点

### serdeのenum discriminant

タグベースのデシリアライズを使用:
```rust
#[derive(Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum CodexRecord {
    SessionMeta(SessionMetaRecord),
    ResponseItem(ResponseItemRecord),
    ...
}
```

### Unknownバリアントの扱い

将来の拡張性のため:
```rust
#[serde(other)]
Unknown,
```

### Optional vs Required

- 必須フィールド: そのまま
- オプショナル: `Option<T>`
- デフォルト値: `#[serde(default)]`

## 参考情報

### ファイル構成

```
src/providers/
├── gemini/
│   ├── mod.rs
│   ├── schema.rs      ✅ 完成
│   ├── io.rs          ✅ 完成
│   └── mapper.rs      ✅ 完成
├── codex/
│   ├── mod.rs
│   ├── schema.rs      ✅ 完成
│   ├── io.rs          🚧 未完了
│   └── mapper.rs      🚧 未完了
└── claude/
    ├── mod.rs
    ├── schema.rs      ❌ 未着手
    ├── io.rs          ❌ 未着手
    └── mapper.rs      ❌ 未着手
```

### テストデータ

```
samples-tmp/
├── .gemini/tmp/<hash>/chats/session-*.json
├── .codex/sessions/<date>/rollout-*.jsonl
└── .claude/projects/<encoded>/7f2abd2d-*.jsonl
```

### 関連ドキュメント

- `docs/architecture.md`: v2.0 Pointer Editionアーキテクチャ
- `docs/database_schema.md`: SQLiteスキーマ
- `docs/agtrace_cli_spec.md`: CLI仕様

## 推定工数

- Codex実装: 2-3時間
- Claude実装: 4-6時間（最も複雑）
- テスト・検証: 1-2時間

**合計**: 7-11時間

## 期待される効果

### Before (Value使用)
- コンパイル時エラー検出: ❌
- コード行数: 1000行
- 可読性: ⭐⭐
- 保守性: ⭐⭐

### After (Schema使用)
- コンパイル時エラー検出: ✅
- コード行数: ~800行（推定20%削減）
- 可読性: ⭐⭐⭐⭐
- 保守性: ⭐⭐⭐⭐⭐

---

**最終更新**: 2025-12-10
**担当者**: Claude Code
