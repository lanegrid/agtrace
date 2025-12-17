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

### Phase 3: 4層アーキテクチャの完成（完了）

**コミット**:
- `ca8da25 refactor: implement 4-layer view architecture with display models and views`
- `0140eac refactor: move session display modules from output/ to views/session/`
- `2518500 refactor: move pack and doctor display modules from output/ to views/`

#### 実施内容

**1. 旧コードの削除と統一**
- `output/compact.rs` (263行) 削除
- `pack_view.rs` の `format_compact` を新版に統一

**2. Display Model の整理**
- `display_model.rs` (432行) → `display_model/` ディレクトリに分割
  - `session.rs` - セッション表示モデル (432行)
  - `provider.rs` - プロバイダースキーマ表示モデル (478行、新規)
  - `mod.rs` - モジュール統合

**3. View Layer の構築**
- `views/` ディレクトリを新規作成し、すべての表示ロジックを移動：

**Session views:**
- `output/session_display.rs` → `views/session/compact.rs` (420行)
- `output/timeline.rs` → `views/session/timeline.rs` (384行)
- `reactors/tui_renderer.rs` から `print_event` を抽出 → `views/session/event.rs` (126行)

**Provider views:**
- `handlers/provider_schema.rs` (186行) から表示ロジックを分離
  - → `views/provider/schema.rs` (65行)
  - ハンドラーは10行に削減（94%削減）

**Pack views:**
- `output/pack_view.rs` → `views/pack.rs` (71行)
- 関数名を統一: `output_*` → `print_*`

**Doctor views:**
- `output/doctor_view.rs` → `views/doctor.rs` (78行)

**4. Reactor の簡素化**
- `reactors/tui_renderer.rs`: 214行 → 76行（64%削減）
  - 表示ロジックを `views/session/event.rs` に移動

#### 結果

**アーキテクチャの完成:**
```
Handler → View → DisplayModel → Domain
  (10行)  (65行)   (478行)      (engine)
```

**コード削減:**
- `provider_schema.rs`: 186行 → 10行 (94%削減)
- `tui_renderer.rs`: 214行 → 76行 (64%削減)
- `output/` ディレクトリ: 1,500+行 → 6行（re-exportのみ）

**ファイル構成:**
- Display models: 3ファイル (915行)
- Views: 9ファイル (1,290行)
- すべてのテスト: 167個 ✅ パス

**命名規則の統一:**
- `format_*()`: `Vec<String>`を返す（テスト可能）
- `print_*()`: 直接printlnする（シンプル表示）

---

## 現状分析（2025-12-17 - Phase 3完了後）

### 📊 完成した View Layer

#### views/ モジュール（統一された表示関数） ✅

**Session views:**
1. `views/session/compact.rs::format_compact()` - Compact形式（`Vec<String>`返却）
2. `views/session/compact.rs::format_token_summary()` - トークンサマリー（`Vec<String>`返却）
3. `views/session/timeline.rs::print_events_timeline()` - タイムライン形式
4. `views/session/event.rs::print_event()` - リアルタイムイベント表示

**Provider views:**
5. `views/provider/schema.rs::print_provider_schema()` - スキーマ表示

**Pack views:**
6. `views/pack.rs::print_diagnose()` - Pack診断
7. `views/pack.rs::print_tools()` - Packツール一覧
8. `views/pack.rs::print_compact()` - Pack compact

**Doctor views:**
9. `views/doctor.rs::print_results()` - Doctor結果

#### output/ モジュール（後方互換性のみ）

- `output/mod.rs` - `views/` への再エクスポートのみ（6行）

#### handlers/ での直接表示（残りのprintln数）

```
init.rs:                35回 ⚠️  次の候補
doctor_check.rs:        32回 ⚠️  次の候補
watch.rs:               22回
provider.rs:             7回
project.rs:              7回
index.rs:                7回
doctor_inspect.rs:       7回
corpus_overview.rs:      6回
session_show.rs:         4回
session_list.rs:         3回
pack.rs:                 0回 ✅ (views使用)
lab_export.rs:           1回
doctor_run.rs:           0回 ✅ (views使用)
provider_schema.rs:      0回 ✅ (views使用)
```

---

## 達成した改善 ✅

### ✅ 重複削除完了
- `format_session_compact` (旧) 削除済み
- すべて `format_compact` (新) に統一

### ✅ 表示ロジックの集約完了
- `provider_schema.rs`: 186行 → 10行（94%削減）
- `tui_renderer.rs`: 214行 → 76行（64%削減）
- すべての表示ロジックが `views/` に集約

### ✅ 一貫性の確立
- 命名規則が統一（`format_*` / `print_*`）
- 型安全な DisplayModel を使用
- 4層アーキテクチャの完成

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
- [x] `format_session_compact` (旧) を削除 → `format_compact` (新) に統一 ✅

### 2. 巨大な handler を分割
- [x] `provider_schema.rs` (124 println) → `views/provider/schema.rs` + DisplayModel ✅

### 3. TuiRenderer の表示ロジックを分離
- [x] `TuiRenderer::print_event` → `views/session/event.rs` ✅

### 4. 命名規則を統一 ✅
- `format_*()` → `Vec<String>` を返す（テスト可能）
- `print_*()` → 直接 println（シンプルな表示）
- ~~`render_*()`~~ → 削除（format か print に統一）

### 5. ディレクトリ構造の完全移行 ✅
```
./crates/agtrace-cli/src/
├── display_model/                    # 表示用モデル ✅
│   ├── mod.rs
│   ├── session.rs                   # SessionDisplay
│   └── provider.rs                  # ProviderSchemaDisplay
├── views/                            # 表示実装 ✅
│   ├── mod.rs
│   ├── session/
│   │   ├── compact.rs               # format_compact, format_token_summary
│   │   ├── timeline.rs              # print_events_timeline
│   │   └── event.rs                 # print_event (TuiRenderer用)
│   ├── provider/
│   │   ├── mod.rs
│   │   └── schema.rs                # print_provider_schema
│   ├── pack.rs                      # print_compact, print_diagnose, print_tools
│   └── doctor.rs                    # print_results
├── output/                           # 後方互換性のみ（re-export）
│   └── mod.rs                       # views/ への再エクスポート
└── handlers/                         # ビジネスロジックのみ
```

---

## 完了したタスク ✅

### Phase 1 & 2 (初期実装)
- [x] SessionDisplay 型の作成
- [x] sessions show で SessionDisplay を使用
- [x] watch で SessionDisplay を使用
- [x] TuiRenderer で format_token_summary を使用
- [x] CLAUDE.md に設計原則を追加

### Phase 3 (4層アーキテクチャ完成)
- [x] 旧 `format_session_compact` を削除 → `format_compact` (新) に統一
- [x] `provider_schema.rs` の表示ロジックを分離 (186行 → 10行)
- [x] TuiRenderer の `print_event` を views/ に移動 (214行 → 76行)
- [x] Display Model のディレクトリ化 (`display_model/`)
- [x] Session views の移行 (`views/session/`)
- [x] Provider views の作成 (`views/provider/`)
- [x] Pack views の移行 (`views/pack.rs`)
- [x] Doctor views の移行 (`views/doctor.rs`)
- [x] 命名規則の統一 (`format_*` / `print_*`)
- [x] `output/` の簡素化（re-exportのみ）

**成果:**
- Display models: 3ファイル (915行)
- Views: 9ファイル (1,290行)
- すべてのテスト: 167個パス ✅
- Handler の簡素化: 平均64-94%のコード削減

---

## 次のステップ（優先順位）

### Phase 4 候補: 残りの Handler の表示ロジック移行

1. **中**: `doctor_check.rs` (32 println) → views/ に移行
2. **中**: `init.rs` (35 println) → views/ に移行
3. **低**: その他の小規模ハンドラー (< 10 println)

### その他の改善案

4. **低**: `output/` ディレクトリの完全削除（直接 `views/` インポートに移行）
5. **低**: View Layer のユニットテスト追加（`format_*` 関数）
