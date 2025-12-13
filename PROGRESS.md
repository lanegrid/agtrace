# Implementation Progress: v0 UX (project corpus / lens / pack)

## Phase 1: Engine - Span IR (最重要) ✅

### 1.1 Span型定義
- [x] Create `crates/agtrace-engine/src/span.rs`
- [x] Define `Span` struct with user/assistant/tools/reasoning/system/stats
- [x] Define `Message` struct
- [x] Define `ToolAction` struct
- [x] Define `SpanStats` struct
- [x] Add Serialize/Deserialize derives

### 1.2 Span構築ロジック
- [x] Implement `build_spans(events: &[AgentEventV1]) -> Vec<Span>`
- [x] Implement UserMessage-based span splitting logic
- [x] Implement ToolCall/ToolResult matching by tool_call_id
- [x] Implement SpanStats calculation (pre_tool_ms, tool_ms, post_tool_ms, e2e_ms)
- [x] Handle edge cases (missing tool_call_id, orphaned events)

### 1.3 公開API
- [x] Add `pub mod span;` to `crates/agtrace-engine/src/lib.rs`
- [x] Add `pub use span::{Span, build_spans};` to lib.rs
- [x] Run `cargo build` and fix compilation errors
- [x] Run `cargo clippy` and fix warnings
- [x] Run `cargo fmt`

### 1.4 テスト
- [x] Create basic test for `build_spans` with fixture events
- [x] Run `cargo test` and verify passing

## Phase 2: CLI - Pack Command (基本)

### 2.1 Args拡張
- [ ] Add `Pack` variant to `Commands` enum in `args.rs`
- [ ] Add `template: Option<String>` field (default: "compact")
- [ ] Add `limit: usize` field (default: 20)
- [ ] Run `cargo build` and verify args parsing

### 2.2 Handlers準備
- [ ] Create `crates/agtrace-cli/src/handlers/pack.rs`
- [ ] Add `pub mod pack;` to `handlers/mod.rs`
- [ ] Implement basic `handle()` function skeleton
- [ ] Connect `Commands::Pack` to `handlers::pack::handle()` in `commands.rs`

### 2.3 SessionDigest作成
- [ ] Define `SessionDigest` struct in `pack.rs` or separate module
- [ ] Implement opening/activation/outcome extraction
- [ ] Implement activation detection logic (tool_calls >= 3 in next 5 spans)
- [ ] Implement importance score calculation

### 2.4 Pack Compact実装
- [ ] Implement session loading for multiple sessions
- [ ] Implement `build_spans()` integration
- [ ] Implement SessionDigest calculation for each session
- [ ] Implement importance-based sorting
- [ ] Implement limit-based filtering
- [ ] Implement compact template output format
- [ ] Run `cargo build` and fix errors
- [ ] Run `cargo clippy` and fix warnings
- [ ] Run `cargo fmt`

### 2.5 テスト
- [ ] Manual test: `cargo run -- pack`
- [ ] Manual test: `cargo run -- pack --limit 10`
- [ ] Verify output format
- [ ] Update help snapshots if needed

## Phase 3: CLI - Corpus Overview (デフォルト変更)

### 3.1 Overview Handler作成
- [ ] Create `crates/agtrace-cli/src/handlers/corpus_overview.rs`
- [ ] Add `pub mod corpus_overview;` to `handlers/mod.rs`
- [ ] Implement `handle()` function

### 3.2 Overview表示ロジック
- [ ] Implement scope determination (project_hash/all_projects)
- [ ] Load sessions (limit ~50 for overview)
- [ ] Build spans and digests for each session
- [ ] Group by lens (Failures/Bottlenecks/Toolchains/etc.)
- [ ] Display count + 1 representative example per lens
- [ ] Run `cargo build` and fix errors
- [ ] Run `cargo clippy` and fix warnings
- [ ] Run `cargo fmt`

### 3.3 デフォルト動作変更
- [ ] Modify `commands.rs` to call `corpus_overview::handle()` when no subcommand
- [ ] Check if DB exists and has sessions before showing overview
- [ ] Fall back to guidance if no sessions
- [ ] Run `cargo build` and verify
- [ ] Manual test: `cargo run --` (no subcommand)

### 3.4 テスト
- [ ] Manual test with existing sessions
- [ ] Manual test with empty DB
- [ ] Update help snapshots if needed

## Phase 4: Pack Template拡張

### 4.1 Diagnose Template
- [ ] Implement `pack diagnose` template
- [ ] Group sessions by lens chapters (Failures/Bottlenecks/Toolchains/Loops)
- [ ] Format output with chapter structure
- [ ] Manual test: `cargo run -- pack diagnose`

### 4.2 Tools Template
- [ ] Implement `pack tools` template
- [ ] Emphasize Toolchains/Bottlenecks chapters
- [ ] Format output
- [ ] Manual test: `cargo run -- pack tools`

### 4.3 品質確認
- [ ] Run `cargo clippy` and fix all warnings
- [ ] Run `cargo fmt`
- [ ] Run `cargo test`
- [ ] Update help snapshots

## Phase 5: Compact出力リファクタ

### 5.1 文字列生成関数の分離
- [ ] Read `crates/agtrace-cli/src/output/compact.rs`
- [ ] Create `format_turns_compact(turns, opts) -> Vec<String>`
- [ ] Refactor `print_turns_compact()` to use `format_*`
- [ ] Create `format_spans_compact(spans, opts) -> Vec<String>`
- [ ] Create `format_session_compact(digest, spans, opts) -> String`
- [ ] Run `cargo build` and fix errors
- [ ] Run `cargo clippy`
- [ ] Run `cargo fmt`

### 5.2 Pack統合
- [ ] Update `pack.rs` to use new format functions
- [ ] Ensure ANSI-free output for pack (paste-friendly)
- [ ] Ensure no relative timestamps in pack output
- [ ] Manual test: verify pack output matches expected format

### 5.3 テスト
- [ ] Run `cargo test`
- [ ] Manual regression test: `cargo run -- session show <id> --style compact`
- [ ] Verify no breaking changes

## Phase 6: Session Show Spanベース移行 (任意)

- [ ] Evaluate if needed for v0
- [ ] If proceeding: update `session show --style compact` to use `build_spans`
- [ ] Ensure consistency between pack and session compact output
- [ ] Run tests and verify

## Final Quality Checks

- [ ] Run `cargo clippy --all-targets` and ensure no warnings
- [ ] Run `cargo fmt --all` and verify formatting
- [ ] Run `cargo test --all` and ensure all tests pass
- [ ] Run `cargo build --release` and verify release build
- [ ] Manual E2E testing of all new commands
- [ ] Review all commits (ensure oneline format)
- [ ] Update documentation if needed

## Notes

- Commit frequently with oneline messages (no co-author, no multiline)
- Run clippy/fmt before each commit
- Test incrementally at each phase
- Update this file as tasks are completed
