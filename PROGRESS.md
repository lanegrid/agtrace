# View整理タスク - 進捗

## 背景・目的

### なぜ整理が必要になったか

agtraceのCLIでは、セッション情報を複数のコマンドで表示する必要がある：
- `agtrace session show` - セッションの静的表示
- `agtrace watch` - セッションのリアルタイム監視
- その他（provider schema、pack、doctorなど）

当初は各コマンドが独自に表示ロジックを実装していたため、以下の問題が発生：

1. **表示の不整合**: watchとsession showで同じセッション情報を表示しても、フォーマットや表示内容が異なる
2. **バグの温床**: token usage表示で`cache_read_tokens`を忘れるなど、型レベルで保証されていない
3. **保守コストの増大**: 表示ロジックが各ハンドラーに散在（provider_schema.rsに124回のprintln）

### 設計方針

**「型安全な表示モデルで統一する」**

従来のアプローチ：
```rust
// ❌ 各ハンドラーがドメインモデルを直接表示
fn handle(session: &AgentSession) {
    for turn in &session.turns {
        println!("User: {}", turn.user.text);  // バラバラ
    }
}
```

新しいアプローチ：
```rust
// ✅ 表示用モデルを経由して統一
fn handle(session: &AgentSession) {
    let display = SessionDisplay::from_agent_session(session);
    let lines = format_compact(&display, &opts);
    for line in lines { println!("{}", line); }
}
```

これにより：
- 表示に必要な情報が型レベルで保証される
- 表示ロジックが一箇所に集約される
- テストが容易になる（`Vec<String>`を返すので比較可能）

---

## 実施済みの作業（時系列）

### Phase 1: SessionDisplay の導入（完了）

**コミット**: `4cf0483 feat: add unified SessionDisplay for type-safe session rendering`

作成したファイル：
- `./crates/agtrace-cli/src/display_model.rs` - SessionDisplay等の型定義
- `./crates/agtrace-cli/src/output/session_display.rs` - 統一された表示関数

実装内容：
- `SessionDisplay` - セッション全体の表示モデル
- `TurnDisplay` - 1ターンの表示情報
- `StepDisplay` / `StepContent` - ステップの表示内容（Reasoning, Tools, Message）
- `ToolDisplay` - ツール実行の表示情報
- `TokenSummaryDisplay` - トークンサマリー
- `format_compact()` - compact形式での表示関数

更新したハンドラー：
- `./crates/agtrace-cli/src/handlers/session_show.rs` - SessionDisplayを使用
- `./crates/agtrace-cli/src/handlers/watch.rs` - 初期表示でSessionDisplayを使用

結果：
- session showとwatchの初期表示が統一された
- 旧`format_session_compact`と新`format_compact`が共存（重複）

### Phase 2: TuiRendererのトークン表示統一（完了）

**コミット**: `0eda494 refactor: unify token display using format_token_summary in TuiRenderer`

問題：
- `TuiRenderer`が独自のトークン表示ロジックを持っていた
- `format_token_summary`が作成されたが使われていなかった（警告が出ていた）

実施内容：
- `TokenSummaryDisplay`に`model`フィールドを追加
- `TuiRenderer::print_token_summary`を`format_token_summary`を使うように変更
- 重複していたヘルパー関数（`create_claude_style_bar`, `format_token_count`）を削除

結果：
- watchのリアルタイムトークン表示も統一された
- コード量削減: 110行削除、36行追加（-74行）
- すべての警告が解消

---

## 現状分析（2025-12-17）

### 📊 存在するView一覧

#### output/ モジュール（公式な表示関数）

**セッション表示系（重複あり）**
1. `compact.rs::format_session_compact()` - **旧** AgentSession用compact
2. `session_display.rs::format_compact()` - **新** SessionDisplay用compact ⭐
3. `session_display.rs::format_token_summary()` - トークンサマリー ⭐
4. `timeline.rs::print_events_timeline()` - タイムライン形式

**Pack表示系**
5. `pack_view.rs::output_diagnose()` - Pack診断
6. `pack_view.rs::output_tools()` - Packツール一覧
7. `pack_view.rs::output_compact()` - Pack compact

**その他**
8. `doctor_view.rs::print_results()` - Doctor結果

#### handlers/ での直接表示（println の数）

```
provider_schema.rs:    124回 ⚠️  異常に多い
init.rs:                35回
doctor_check.rs:        32回
watch.rs:               22回
provider.rs:             7回
project.rs:              7回
index.rs:                7回
doctor_inspect.rs:       7回
corpus_overview.rs:      6回
session_show.rs:         4回
session_list.rs:         3回
pack.rs:                 1回
lab_export.rs:           1回
doctor_run.rs:           1回
```

#### reactors/ での表示

9. `TuiRenderer::print_event()` - リアルタイムイベント表示

---

## 問題点

### ❌ 重複
- `format_session_compact` (旧) と `format_compact` (新) が共存

### ❌ 散乱
- `provider_schema.rs` で124回もprintln（表示ロジックが混在）
- handlers が直接 println を使っている

### ❌ 一貫性なし
- 命名規則が統一されていない（format, print, output）
- TuiRenderer が独自の表示ロジックを持っている
- 旧型（AgentSession）と新型（SessionDisplay）が混在

---

## 提案：4層アーキテクチャ

```
┌─────────────────────────────────────┐
│ Handler Layer (handlers/)           │ ← CLI引数を受け取り、viewを呼ぶだけ
│ 責務: コマンド処理、引数パース        │
└─────────────────────────────────────┘
           ↓
┌─────────────────────────────────────┐
│ View Layer (output/ → views/)       │ ← 表示形式の実装
│ 責務: DisplayModel を受け取り表示     │
│ - session/compact.rs                │
│ - session/timeline.rs               │
│ - pack/diagnose.rs                  │
│ - provider/schema.rs                │
└─────────────────────────────────────┘
           ↓
┌─────────────────────────────────────┐
│ Presentation Layer (display_model)  │ ← 表示用モデル
│ 責務: ドメインモデル → 表示用に変換   │
│ - SessionDisplay                    │
│ - ProviderSchemaDisplay             │
│ - PackDisplay                       │
└─────────────────────────────────────┘
           ↓
┌─────────────────────────────────────┐
│ Domain Layer (agtrace-engine)       │ ← ビジネスロジック
│ 責務: データ構造、ドメインロジック     │
│ - AgentSession                      │
│ - AgentEvent                        │
└─────────────────────────────────────┘
```

---

## 整理方針

### 1. 重複を削除
- [ ] `format_session_compact` (旧) を削除 → `format_compact` (新) に統一

### 2. 巨大な handler を分割
- [ ] `provider_schema.rs` (124 println) → `views/provider/schema.rs` + DisplayModel

### 3. TuiRenderer の表示ロジックを分離
- [ ] `TuiRenderer::print_event` → `views/session/event.rs`

### 4. 命名規則を統一
- `format_*()` → `Vec<String>` を返す（テスト可能）
- `print_*()` → 直接 println（シンプルな表示）
- ~~`render_*()`~~ → 削除（format か print に統一）

### 5. ディレクトリ構造（プロジェクトルートからの相対パス）
```
./crates/agtrace-cli/src/
├── display_model/                    # 表示用モデル
│   ├── mod.rs
│   ├── session.rs                   # SessionDisplay
│   ├── provider.rs                  # ProviderSchemaDisplay
│   └── pack.rs                      # PackDisplay
├── views/                            # 表示実装
│   ├── mod.rs
│   ├── session/
│   │   ├── compact.rs               # format_compact
│   │   ├── timeline.rs              # format_timeline
│   │   └── event.rs                 # format_event (TuiRenderer用)
│   ├── provider/
│   │   └── schema.rs                # format_schema
│   ├── pack/
│   │   ├── diagnose.rs
│   │   ├── tools.rs
│   │   └── compact.rs
│   └── doctor/
│       └── results.rs
└── handlers/                         # ビジネスロジックのみ
```

---

## 完了したタスク

- [x] SessionDisplay 型の作成
- [x] sessions show で SessionDisplay を使用
- [x] watch で SessionDisplay を使用
- [x] TuiRenderer で format_token_summary を使用
- [x] CLAUDE.md に設計原則を追加

---

## 次のステップ（優先順位）

1. **高**: 旧 `format_session_compact` を削除
2. **高**: `provider_schema.rs` の表示ロジックを分離
3. **中**: TuiRenderer の `print_event` を views/ に移動
4. **低**: 残りの handlers/ の表示ロジックを段階的に移行
