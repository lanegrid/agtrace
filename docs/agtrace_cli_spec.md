# agtrace CLI Specification (v1)

## 0. Overview

`agtrace` は、Claude Code / Codex / Gemini CLI などのエージェント行動ログを:

1. 正規化スキーマ (`AgentEventV1`) に変換し
2. ローカルストレージ（ファイル / DB）に格納し
3. CLI からセッション一覧・詳細・統計・エクスポートを行う

ための CLI ツールである。

本仕様では、CLI インターフェイス（コマンド・引数・出力）の要件を定義する。

---

## 1. 全体構成

### 1.1 コマンド一覧

`agtrace` は次のサブコマンドを持つ:

* **`agtrace import`**
  ベンダーログを読み込み、`AgentEventV1` に正規化してストレージに取り込む。

* **`agtrace list`**
  セッション / 実行の一覧を表示する。

* **`agtrace show`**
  特定セッションのイベントタイムラインを表示する。

* **`agtrace find`**
  イベント ID やテキストで検索する。

* **`agtrace stats`**
  トークン・ツール呼び出し・セッション数などの統計を集計する。

* **`agtrace export`**
  正規化済みイベントを JSONL / CSV などでエクスポートする。

### 1.2 グローバルオプション

全サブコマンド共通で次のオプションを受け付ける:

* `--data-dir <PATH>`

  * 説明: agtrace がデータ（正規化済みイベント / インデックス / DB）を保存するルートディレクトリ。
  * 既定値: `$HOME/.agtrace`

* `--format <plain|json>`

  * 説明: CLI の標準出力フォーマット。
  * `list` / `show` / `stats` などに適用される。
  * 既定値: `plain`

* `--log-level <error|warn|info|debug|trace>`

  * 説明: CLI 自身のログレベル。
  * 既定値: `info`

* `--version`

  * 説明: agtrace のバージョンを表示する。

* `--help`

  * 説明: ヘルプを表示する。

---

## 2. `agtrace import` — ベンダーログの取り込み

### 2.1 概要

ベンダー固有のログ（Claude JSONL / Codex rollout JSONL / Gemini JSON）を走査し、
`AgentEventV1` に正規化して `data-dir` 配下に保存する。

### 2.2 シグネチャ

```sh
agtrace import \
  --source <claude|codex|gemini> \
  --root <PATH> \
  [--project-root <PATH>] \
  [--session-id-prefix <STRING>] \
  [--dry-run] \
  [--out-jsonl <PATH>]
```

### 2.3 オプション詳細

* `--source <claude|codex|gemini>` (必須)

  * 説明: 取り込み対象のベンダーを指定する。
  * 対応:

    * `claude`  → Claude Code JSONL (`~/.claude/projects/.../*.jsonl`)
    * `codex`   → Codex rollout JSONL (`~/.codex/sessions/.../rollout-*.jsonl`)
    * `gemini`  → Gemini CLI (`~/.gemini/tmp/**/logs.json` + `chats/*.json`)

* `--root <PATH>` (必須)

  * 説明: ベンダーログのルートディレクトリ。
  * 例:

    * Claude: `~/.claude/projects/-Users-zawakin-go-src-github-com-lanegrid-agtrace`
    * Codex:  `~/.codex/sessions/2025/11`
    * Gemini: `~/.gemini/tmp`

* `--project-root <PATH>` (任意)

  * 説明: プロジェクトのルートパスを明示する。
  * 挙動:

    * 指定された場合、`project_root` として優先的に使用し、`project_hash = sha256(project_root)` とする。
    * 指定されない場合、ベンダーログ内の `cwd` や `projectHash` から推定する。

* `--session-id-prefix <STRING>` (任意)

  * 説明: 生成される `session_id` の先頭に付与する接頭辞。
  * 例: `"codex-2025-11-"` など。

* `--dry-run`

  * 説明: 実際には `data-dir` に書き込まず、何件のセッション・イベントが生成されるかだけを表示する。

* `--out-jsonl <PATH>` (任意)

  * 説明: ストレージに書き込むと同時に、正規化済みイベントを JSONL ファイルとして書き出す。
  * 利用用途: デバッグ / 他ツールへの連携。

### 2.4 入出力・ストレージ

* 入力:

  * ベンダーログファイル
* 処理:

  * 各ファイルを vendor-specific パーサで読み込み
  * `normalize_*` 関数で `Vec<AgentEventV1>` に変換
  * `session_id` / `event_id` / `parent_event_id` を埋める
  * `data-dir` 配下に保存
* 保存形式（例示。実装は以下の条件を満たせば良い）:

  * `data-dir/events/<project_hash>/<session_id>.jsonl`

    * 中身は `AgentEventV1` の 1 行 1 イベント JSON

---

## 3. `agtrace list` — セッション / 実行の一覧

### 3.1 概要

正規化済みデータから、セッション（= `session_id`）ごとの概要を一覧表示する。

### 3.2 シグネチャ

```sh
agtrace list \
  [--project-hash <HASH>] \
  [--source <claude|codex|gemini>] \
  [--limit <N>] \
  [--since <RFC3339>] \
  [--until <RFC3339>]
```

### 3.3 オプション詳細

* `--project-hash <HASH>`

  * 説明: 特定プロジェクトのセッションだけに絞る。

* `--source <claude|codex|gemini>`

  * 説明: 特定ベンダー由来のセッションだけに絞る。

* `--limit <N>`

  * 説明: 表示するセッション数の上限。
  * 既定値: `50`

* `--since <RFC3339>` / `--until <RFC3339>`

  * 説明: `ts` の範囲でフィルタする（セッションの最初のイベント時刻基準）。

### 3.4 出力イメージ（`--format=plain`）

```text
SESSION ID                          SOURCE       PROJECT HASH                         START TIME                EVENTS  USER MSG  TOKENS(in/out)
codex-2025-11-03T01-46-11-0        codex        2e4c1f...                             2025-11-03T01:46:11.987Z  123     8         12_345 /  9_876
claude-2025-11-26T12-51-28-0       claude_code  9a7b3c...                             2025-11-26T12:51:28.093Z  256     15        34_567 / 21_234
gemini-2025-12-07T17-17-16-876Z    gemini       427e6b3f...                           2025-12-07T17:17:16.876Z  42      4         7_200  /  2_000
...
```

### 3.5 出力イメージ（`--format=json`）

```json
[
  {
    "session_id": "codex-2025-11-03T01-46-11-0",
    "source": "codex",
    "project_hash": "2e4c1f...",
    "start_ts": "2025-11-03T01:46:11.987Z",
    "end_ts": "2025-11-03T01:59:30.123Z",
    "event_count": 123,
    "user_message_count": 8,
    "tokens_input_total": 12345,
    "tokens_output_total": 9876
  },
  ...
]
```

---

## 4. `agtrace show` — セッションの詳細タイムライン

### 4.1 概要

特定 `session_id` のイベントを、時系列に沿って表示する。

### 4.2 シグネチャ

```sh
agtrace show <SESSION_ID> \
  [--event-type <TYPE,...>] \
  [--no-reasoning] \
  [--no-tool] \
  [--limit <N>]
```

### 4.3 オプション詳細

* `<SESSION_ID>` (必須)

  * `agtrace list` で表示された `session_id`。

* `--event-type <TYPE,...>`

  * 表示するイベント種別をカンマ区切りで指定。
  * 例: `user_message,assistant_message,tool_call,tool_result`

* `--no-reasoning`

  * `event_type = "reasoning"` を非表示にする。

* `--no-tool`

  * `event_type in {"tool_call","tool_result"}` を非表示にする。

* `--limit <N>`

  * 表示するイベント数の上限。

### 4.4 出力イメージ（`--format=plain`）

```text
[2025-11-03T01:49:22.517Z] user_message       U1   (role=user)
  summary this repo

[2025-11-03T01:49:23.073Z] reasoning          R1   (role=assistant)
  Plan: read README, scan src/, then propose a summary...

[2025-11-03T01:49:25.212Z] tool_call          T1   (shell)
  rg "agtrace" -n

[2025-11-03T01:49:26.836Z] tool_result        TR1  (shell, status=success)
  README.md:1: # agtrace
  ...

[2025-11-03T01:49:30.519Z] assistant_message  A1   (role=assistant)
  This repo is `agtrace`, a Rust CLI/library for unifying agent traces...
```

---

## 5. `agtrace find` — イベント検索

### 5.1 概要

`event_id` やテキストをキーに、イベントを横断検索する。

### 5.2 シグネチャ

```sh
agtrace find \
  [--session-id <SESSION_ID>] \
  [--project-hash <HASH>] \
  [--event-id <EVENT_ID>] \
  [--text <QUERY>] \
  [--event-type <TYPE,...>] \
  [--limit <N>]
```

### 5.3 オプション詳細

* `--session-id <SESSION_ID>`

  * 特定セッション内だけを対象に検索。

* `--project-hash <HASH>`

  * 特定プロジェクトだけを対象に検索。

* `--event-id <EVENT_ID>`

  * event_id を完全一致で検索。

* `--text <QUERY>`

  * `text` に対する部分一致検索。

* `--event-type <TYPE,...>`

  * 検索対象とする event_type の絞り込み。

* `--limit <N>`

  * 最大ヒット数。

### 5.4 出力イメージ

```text
SESSION                        TS                          TYPE              EVENT_ID
codex-2025-11-03T01-46-11-0    2025-11-03T01:49:30.519Z   assistant_message  2025-11-03T01:49:30.519Z#5
  text: This repo is `agtrace`, a Rust CLI/library...

claude-2025-11-26T12-51-28-0   2025-11-26T15:49:10.000Z   tool_call         toolu_01PLVMXZk2vbmGF3GsDu3aGQ
  tool_name: TodoWrite
  text: { "todos": [...] }
```

---

## 6. `agtrace stats` — 統計・メトリクス

### 6.1 概要

プロジェクト / セッション / ベンダーごとの統計情報を集計して表示する。

### 6.2 シグネチャ

```sh
agtrace stats \
  [--project-hash <HASH>] \
  [--source <claude|codex|gemini>] \
  [--group-by <project|session|source>] \
  [--since <RFC3339>] \
  [--until <RFC3339>]
```

### 6.3 オプション詳細

* `--project-hash <HASH>`

  * 特定プロジェクトのみ集計。

* `--source <claude|codex|gemini>`

  * 特定ベンダーのみ集計。

* `--group-by <project|session|source>`

  * 集計の粒度を指定。
  * 例:

    * `project`: プロジェクトごとの合計トークン・セッション数
    * `session`: セッションごとのトークン・ツール使用回数
    * `source`: ベンダーごとの比較

### 6.4 出力イメージ（`--group-by=project`）

```text
PROJECT HASH                         SESSIONS  EVENTS  USER MSG  TOOL CALLS  TOKENS(in/out)
427e6b3f... (agtrace)               12        1_234   92        210         345_678 / 210_987
2e4c1f...  (transcene)              5         456     30        88          120_000 /  80_000
...
```

---

## 7. `agtrace export` — エクスポート

### 7.1 概要

正規化済み `AgentEventV1` を JSONL / CSV にエクスポートする。

### 7.2 シグネチャ

```sh
agtrace export \
  [--project-hash <HASH>] \
  [--session-id <SESSION_ID>] \
  [--source <claude|codex|gemini>] \
  [--event-type <TYPE,...>] \
  [--since <RFC3339>] \
  [--until <RFC3339>] \
  [--out <PATH>] \
  [--format <jsonl|csv>]
```

### 7.3 オプション詳細

* `--out <PATH>` (必須)

  * 出力先ファイルパス。

* `--format <jsonl|csv>`

  * 既定値: `jsonl`

* 他のフィルタオプションは `find` / `stats` と同様。

---

## 8. エラーコード・終了ステータス

* `0` … 正常終了
* `1` … 一般的なエラー（パース失敗 / 入力不正など）
* `2` … 入力パスが存在しない / 読み取り不能
* `3` … ストレージ書き込みエラー
* `4` … 内部エラー（バグ）

---

## 9. 今後の拡張余地（非必須）

* `agtrace graph`

  * セッション中の user → reasoning → tool → result → assistant の DAG を Graphviz 等にエクスポート。
* `agtrace diff`

  * 2 つのセッションの行動差分を比較。
* `agtrace serve`

  * Web UI を立ち上げ、ブラウザから可視化。

---

以上が **agtrace CLI v1** の仕様である。
この仕様に沿って CLI 実装を進めれば、正規化済みスキーマ `AgentEventV1` と自然に整合するはずである。
