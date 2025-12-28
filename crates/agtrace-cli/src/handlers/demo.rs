use crate::presentation::presenters::watch_tui::build_screen_view_model;
use crate::presentation::renderers::tui::{RendererSignal, TuiEvent, TuiRenderer};
use agtrace_engine::session::assemble_session;
use agtrace_engine::token_usage::ContextWindowUsage;
use agtrace_runtime::SessionState;
use agtrace_types::{
    AgentEvent, EventPayload, ExecuteArgs, FileEditArgs, FileReadArgs, MessagePayload,
    ReasoningPayload, StreamId, ToolCallPayload, ToolResultPayload, UserPayload,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

struct DemoConfig {
    step_delay: u64,
}

impl DemoConfig {
    fn from_str(s: &str) -> Self {
        let step_delay = match s {
            "slow" => 1500,
            "fast" => 100,
            _ => 500,
        };
        Self { step_delay }
    }
}

pub fn handle(speed: String) -> Result<()> {
    let config = DemoConfig::from_str(&speed);

    let (event_tx, event_rx) = mpsc::channel();
    let (signal_tx, signal_rx) = mpsc::channel();

    let tui_handle = thread::spawn(move || {
        let renderer = TuiRenderer::new().with_signal_sender(signal_tx);
        renderer.run(event_rx)
    });

    let result = run_simulation(config, event_tx, signal_rx);

    if let Err(e) = tui_handle.join() {
        eprintln!("TUI thread panicked: {:?}", e);
    }

    result
}

fn run_simulation(
    config: DemoConfig,
    tx: mpsc::Sender<TuiEvent>,
    signal_rx: mpsc::Receiver<RendererSignal>,
) -> Result<()> {
    let session_id = "demo-session-00000000-0000-0000-0000-000000000000".to_string();
    let start_time = Utc::now();

    let mut state = SessionState::new(session_id.clone(), None, start_time);
    state.context_window_limit = Some(100_000);
    state.model = Some("claude-3-5-sonnet-20241022 (Demo)".to_string());

    let mut events_buffer = VecDeque::new();

    let scenario = generate_scenario(&session_id, start_time);

    for (idx, event) in scenario.into_iter().enumerate() {
        match signal_rx.try_recv() {
            Ok(RendererSignal::Quit) => break,
            Err(mpsc::TryRecvError::Disconnected) => break,
            _ => {}
        }

        state.last_activity = event.timestamp;
        state.event_count += 1;

        if let EventPayload::User(_) = &event.payload {
            state.turn_count += 1;
        }

        match &event.payload {
            EventPayload::ToolResult(_) | EventPayload::Message(_) => {
                let mut input_tokens = state.current_usage.input_tokens() + 2000;
                let output_tokens = state.current_usage.output_tokens() + 100;

                if idx == 10 {
                    input_tokens += 50000;
                }

                state.current_usage =
                    ContextWindowUsage::from_raw(input_tokens, 0, 0, output_tokens);
            }
            _ => {}
        }

        events_buffer.push_back(event);
        if events_buffer.len() > 100 {
            events_buffer.pop_front();
        }

        let max_context = state.context_window_limit.map(|x| x as u32);

        let notification = if idx == 0 {
            Some("DEMO MODE: Simulating active session...".to_string())
        } else if idx == 10 {
            Some("Warning: Large file detected, context usage spiked!".to_string())
        } else {
            None
        };

        let events_vec: Vec<_> = events_buffer.iter().cloned().collect();
        let assembled_session = assemble_session(&events_vec);

        let vm = build_screen_view_model(
            &state,
            &events_buffer,
            assembled_session.as_ref(),
            max_context,
            notification.as_deref(),
        );

        if tx.send(TuiEvent::Update(Box::new(vm))).is_err() {
            break;
        }

        thread::sleep(Duration::from_millis(config.step_delay));
    }

    thread::sleep(Duration::from_secs(2));
    Ok(())
}

struct ScenarioBuilder {
    events: Vec<AgentEvent>,
    session_uuid: uuid::Uuid,
    stream_id: StreamId,
    timestamp: DateTime<Utc>,
    event_counter: u32,
    call_counter: u32,
}

impl ScenarioBuilder {
    fn new(session_id: &str, start: DateTime<Utc>) -> Self {
        use std::str::FromStr;
        let session_uuid = uuid::Uuid::from_str(session_id)
            .unwrap_or_else(|_| uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap());

        Self {
            events: Vec::new(),
            session_uuid,
            stream_id: StreamId::Main,
            timestamp: start,
            event_counter: 0,
            call_counter: 0,
        }
    }

    fn tick(&mut self) -> DateTime<Utc> {
        self.timestamp += chrono::Duration::seconds(2);
        self.timestamp
    }

    fn next_event_id(&mut self) -> uuid::Uuid {
        self.event_counter += 1;
        uuid::Uuid::from_u128(self.event_counter as u128)
    }

    fn next_call_id(&mut self) -> String {
        self.call_counter += 1;
        format!("call_{:02}", self.call_counter)
    }

    fn user(&mut self, text: impl Into<String>) -> &mut Self {
        let id = self.next_event_id();
        let ts = self.tick();
        self.events.push(AgentEvent {
            id,
            session_id: self.session_uuid,
            parent_id: None,
            timestamp: ts,
            stream_id: self.stream_id.clone(),
            metadata: None,
            payload: EventPayload::User(UserPayload { text: text.into() }),
        });
        self
    }

    fn think(&mut self, text: impl Into<String>) -> &mut Self {
        let id = self.next_event_id();
        let ts = self.tick();
        self.events.push(AgentEvent {
            id,
            session_id: self.session_uuid,
            parent_id: None,
            timestamp: ts,
            stream_id: self.stream_id.clone(),
            metadata: None,
            payload: EventPayload::Reasoning(ReasoningPayload { text: text.into() }),
        });
        self
    }

    fn read_file(&mut self, path: impl Into<String>, output: impl Into<String>) -> &mut Self {
        let call_id = self.next_event_id();
        let ts1 = self.tick();
        let provider_id = self.next_call_id();
        let path_str = path.into();
        self.events.push(AgentEvent {
            id: call_id,
            session_id: self.session_uuid,
            parent_id: None,
            timestamp: ts1,
            stream_id: self.stream_id.clone(),
            metadata: None,
            payload: EventPayload::ToolCall(ToolCallPayload::FileRead {
                name: "Read".to_string(),
                arguments: FileReadArgs {
                    file_path: Some(path_str),
                    path: None,
                    pattern: None,
                    extra: serde_json::json!({}),
                },
                provider_call_id: Some(provider_id),
            }),
        });
        let result_id = self.next_event_id();
        let ts2 = self.tick();
        self.events.push(AgentEvent {
            id: result_id,
            session_id: self.session_uuid,
            parent_id: None,
            timestamp: ts2,
            stream_id: self.stream_id.clone(),
            metadata: None,
            payload: EventPayload::ToolResult(ToolResultPayload {
                output: output.into(),
                tool_call_id: call_id,
                is_error: false,
            }),
        });
        self
    }

    fn edit_file(&mut self, path: impl Into<String>, old: impl Into<String>, new: impl Into<String>, count: usize) -> &mut Self {
        let call_id = self.next_event_id();
        let ts1 = self.tick();
        let provider_id = self.next_call_id();
        let (path_str, old_str, new_str) = (path.into(), old.into(), new.into());
        self.events.push(AgentEvent {
            id: call_id,
            session_id: self.session_uuid,
            parent_id: None,
            timestamp: ts1,
            stream_id: self.stream_id.clone(),
            metadata: None,
            payload: EventPayload::ToolCall(ToolCallPayload::FileEdit {
                name: "Edit".to_string(),
                arguments: FileEditArgs {
                    file_path: path_str,
                    old_string: old_str,
                    new_string: new_str,
                    replace_all: count > 1,
                },
                provider_call_id: Some(provider_id),
            }),
        });
        let result_id = self.next_event_id();
        let ts2 = self.tick();
        self.events.push(AgentEvent {
            id: result_id,
            session_id: self.session_uuid,
            parent_id: None,
            timestamp: ts2,
            stream_id: self.stream_id.clone(),
            metadata: None,
            payload: EventPayload::ToolResult(ToolResultPayload {
                output: format!("Applied {} edit{}.", count, if count == 1 { "" } else { "s" }),
                tool_call_id: call_id,
                is_error: false,
            }),
        });
        self
    }

    fn bash(&mut self, cmd: impl Into<String>, is_error: bool, output: impl Into<String>) -> &mut Self {
        let call_id = self.next_event_id();
        let ts1 = self.tick();
        let provider_id = self.next_call_id();
        let cmd_str = cmd.into();
        self.events.push(AgentEvent {
            id: call_id,
            session_id: self.session_uuid,
            parent_id: None,
            timestamp: ts1,
            stream_id: self.stream_id.clone(),
            metadata: None,
            payload: EventPayload::ToolCall(ToolCallPayload::Execute {
                name: "Bash".to_string(),
                arguments: ExecuteArgs {
                    command: Some(cmd_str),
                    description: None,
                    timeout: None,
                    extra: serde_json::json!({}),
                },
                provider_call_id: Some(provider_id),
            }),
        });
        let result_id = self.next_event_id();
        let ts2 = self.tick();
        self.events.push(AgentEvent {
            id: result_id,
            session_id: self.session_uuid,
            parent_id: None,
            timestamp: ts2,
            stream_id: self.stream_id.clone(),
            metadata: None,
            payload: EventPayload::ToolResult(ToolResultPayload {
                output: output.into(),
                tool_call_id: call_id,
                is_error,
            }),
        });
        self
    }

    fn message(&mut self, text: impl Into<String>) -> &mut Self {
        let id = self.next_event_id();
        let ts = self.tick();
        self.events.push(AgentEvent {
            id,
            session_id: self.session_uuid,
            parent_id: None,
            timestamp: ts,
            stream_id: self.stream_id.clone(),
            metadata: None,
            payload: EventPayload::Message(MessagePayload { text: text.into() }),
        });
        self
    }

    fn build(self) -> Vec<AgentEvent> {
        self.events
    }
}

fn generate_scenario(session_id: &str, start: DateTime<Utc>) -> Vec<AgentEvent> {
    let mut builder = ScenarioBuilder::new(session_id, start);

    // Turn 1: Error handling improvement
    builder
        .user("src/handlers/index.rs のエラーハンドリングを改善して。anyhowを使ってコンテキストを追加したい。")
        .think("まずは現状のコードを確認する必要がある。src/handlers/index.rs を読み込む。")
        .read_file("src/handlers/index.rs", "pub fn handle() -> Result<()> {\n    let data = std::fs::read_to_string(\"data.json\").unwrap();\n    Ok(())\n}")
        .think("unwrap()が使われている。これをcontext()に置き換える。")
        .edit_file("src/handlers/index.rs", "unwrap()", "context(\"failed to read file\")?", 3)
        .bash("cargo check", true, "error[E0425]: cannot find value `anyhow` in this scope")
        .think("インポートを忘れていた。use anyhow::Context; を追加する。")
        .edit_file("src/handlers/index.rs", "use std::fs;", "use std::fs;\nuse anyhow::Context;", 1)
        .bash("cargo check", false, "    Checking agtrace-cli v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s) in 2.3s")
        .message("修正完了しました。anyhow::Contextを利用してエラー時のコンテキスト情報を付与するように変更しました。");

    // Turn 2: Add tests
    builder
        .user("テストも追加してください")
        .think("まずテストファイルの構造を確認する")
        .bash("ls src/handlers/*.rs | grep test", true, "No such file")
        .think("テストファイルがない。tests/内に作成する")
        .read_file("tests/integration_test.rs", "// Basic test structure...")
        .think("index.rsのテストケースを追加する")
        .edit_file("tests/integration_test.rs", "// Add tests here", "#[test]\nfn test_handle_success() {\n    let result = handle();\n    assert!(result.is_ok());\n}", 1)
        .bash("cargo test handle_success", false, "test test_handle_success ... ok\ntest result: ok. 1 passed; 0 failed")
        .think("エラーケースのテストも追加する")
        .edit_file("tests/integration_test.rs", "}", "#[test]\nfn test_handle_missing_file() {\n    // Test error case\n}\n\n}", 1)
        .bash("cargo test", false, "test result: ok. 2 passed; 0 failed")
        .message("テストを追加しました。成功ケースとエラーケースの両方をカバーしています。");

    // Turn 3: Performance optimization
    builder
        .user("もっと高速化できますか？パフォーマンスが気になります")
        .think("まずベンチマークを取って現状を把握する")
        .bash("cargo build --release", false, "    Compiling agtrace-cli v0.1.0\n    Finished release [optimized] target(s) in 15.2s")
        .bash("hyperfine 'cargo run --release'", false, "Time (mean ± σ):     124.3 ms ±   2.1 ms")
        .think("プロファイリングしてボトルネックを特定する")
        .bash("cargo flamegraph", false, "Wrote flamegraph to flamegraph.svg")
        .think("ファイルI/Oが主なボトルネック。BufReaderを使うと改善できそう")
        .read_file("src/handlers/index.rs", "let data = std::fs::read_to_string(\"data.json\").context(\"failed to read file\")?;")
        .think("BufReaderとキャッシングを導入する")
        .edit_file("src/handlers/index.rs", "use std::fs;", "use std::fs::{self, File};\nuse std::io::{BufReader, BufRead};", 1)
        .edit_file("src/handlers/index.rs", "std::fs::read_to_string(\"data.json\")", "{\n    let file = File::open(\"data.json\")?;\n    let reader = BufReader::new(file);\n    reader.lines().collect::<Result<Vec<_>, _>>()?.join(\"\\n\")\n}", 1)
        .bash("cargo build --release", false, "    Finished release [optimized] target(s) in 3.8s")
        .bash("hyperfine 'cargo run --release'", false, "Time (mean ± σ):      89.1 ms ±   1.8 ms")
        .message("BufReaderを使って約28%高速化しました（124.3ms → 89.1ms）。");

    // Turn 4: Add documentation
    builder
        .user("ドキュメントコメントを追加してください")
        .think("関数のドキュメントを確認する")
        .read_file("src/handlers/index.rs", "pub fn handle() -> Result<()> {")
        .think("docコメントを追加する")
        .edit_file("src/handlers/index.rs", "pub fn handle()", "/// Handles index operations with improved error context.\n///\n/// # Errors\n/// Returns error if file cannot be read or parsed.\n///\n/// # Examples\n/// ```\n/// let result = handle();\n/// assert!(result.is_ok());\n/// ```\npub fn handle()", 1)
        .bash("cargo doc --no-deps", false, "    Documenting agtrace-cli v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s) in 1.2s")
        .think("他の公開関数にもドキュメントを追加する")
        .read_file("src/handlers/index.rs", "pub fn init_index()")
        .edit_file("src/handlers/index.rs", "pub fn init_index()", "/// Initializes the index database.\n/// \n/// # Errors\n/// Returns error if database cannot be created.\npub fn init_index()", 1)
        .bash("cargo doc --open", false, "Opening docs in browser...")
        .message("ドキュメントコメントを追加しました。cargo docで確認できます。");

    // Turn 5: Refactoring
    builder
        .user("コードの重複が気になります。リファクタリングできますか？")
        .think("重複パターンを探す")
        .bash("grep -n 'context(\"failed' src/**/*.rs", false, "src/handlers/index.rs:15:    .context(\"failed to read file\")?;\nsrc/handlers/index.rs:42:    .context(\"failed to parse\")?;")
        .think("共通のエラーハンドリング関数を作る")
        .edit_file("src/handlers/mod.rs", "pub mod index;", "pub mod index;\n\n/// Common error handling utilities\nmod errors {\n    pub fn read_file_error(path: &str) -> String {\n        format!(\"Failed to read file: {}\", path)\n    }\n}", 1)
        .think("index.rsで使うように変更")
        .edit_file("src/handlers/index.rs", "use std::fs", "use std::fs;\nuse super::errors", 1)
        .edit_file("src/handlers/index.rs", ".context(\"failed to read file\")?", ".context(errors::read_file_error(\"data.json\"))?", 1)
        .bash("cargo check", false, "    Checking agtrace-cli v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s) in 1.1s")
        .bash("cargo clippy", false, "    Checking agtrace-cli v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s) in 1.3s")
        .message("エラーハンドリングを共通関数にリファクタリングしました。");

    // Turn 6: Add logging
    builder
        .user("ロギングを追加して、デバッグしやすくしたいです")
        .think("tracing crateを使うのが良さそう")
        .bash("cargo add tracing tracing-subscriber", false, "    Updating crates.io index\n      Adding tracing v0.1.40\n      Adding tracing-subscriber v0.3.18")
        .think("初期化コードを追加")
        .edit_file("src/main.rs", "fn main()", "fn main() {\n    tracing_subscriber::fmt::init();\n", 1)
        .think("handler関数にログを追加")
        .edit_file("src/handlers/index.rs", "use super::errors", "use super::errors;\nuse tracing::{info, debug, error};", 1)
        .edit_file("src/handlers/index.rs", "pub fn handle() -> Result<()> {", "pub fn handle() -> Result<()> {\n    info!(\"Starting handle operation\");", 1)
        .edit_file("src/handlers/index.rs", "let reader = BufReader::new(file);", "let reader = BufReader::new(file);\n    debug!(\"Reading file with BufReader\");", 1)
        .bash("cargo run", false, "2024-01-15T10:23:45.123Z  INFO agtrace_cli::handlers::index: Starting handle operation\n2024-01-15T10:23:45.125Z DEBUG agtrace_cli::handlers::index: Reading file with BufReader")
        .message("tracingを使ったロギングを追加しました。RUST_LOG環境変数で制御できます。");

    // Turn 7: Add configuration
    builder
        .user("設定ファイルで動作をカスタマイズできるようにしてください")
        .think("config.tomlを読む機能を追加する")
        .bash("cargo add serde toml", false, "    Updating crates.io index\n      Adding serde v1.0\n      Adding toml v0.8")
        .think("設定構造体を定義")
        .bash("touch src/config.rs", false, "")
        .edit_file("src/config.rs", "", "use serde::Deserialize;\n\n#[derive(Debug, Deserialize)]\npub struct Config {\n    pub buffer_size: usize,\n    pub enable_cache: bool,\n}", 1)
        .edit_file("src/lib.rs", "pub mod handlers;", "pub mod handlers;\npub mod config;", 1)
        .think("config.tomlから読み込む")
        .bash("echo '[settings]\\nbuffer_size = 8192\\nenable_cache = true' > config.toml", false, "")
        .edit_file("src/handlers/index.rs", "pub fn handle()", "pub fn handle(config: &crate::config::Config)", 1)
        .bash("cargo build", false, "    Compiling agtrace-cli v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s) in 2.1s")
        .message("設定ファイル（config.toml）のサポートを追加しました。");

    builder.build()
}
