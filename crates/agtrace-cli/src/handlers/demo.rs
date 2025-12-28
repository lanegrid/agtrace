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

fn generate_scenario(session_id: &str, start: DateTime<Utc>) -> Vec<AgentEvent> {
    use std::str::FromStr;

    let mut events = Vec::new();
    let mut t = start;
    let stream_id = StreamId::Main;
    let mut event_counter = 0u32;

    let session_uuid = uuid::Uuid::from_str(session_id)
        .unwrap_or_else(|_| uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap());

    let mut tick = || {
        t += chrono::Duration::seconds(2);
        t
    };

    let mut next_event_id = || {
        event_counter += 1;
        uuid::Uuid::from_u128(event_counter as u128)
    };

    events.push(AgentEvent {
        id: next_event_id(),
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::User(UserPayload {
            text: "src/handlers/index.rs のエラーハンドリングを改善して。anyhowを使ってコンテキストを追加したい。".to_string(),
        }),
    });

    events.push(AgentEvent {
        id: next_event_id(),
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::Reasoning(ReasoningPayload {
            text: "ユーザーはエラーハンドリングの改善を求めている。まずは現状のコードを確認する必要がある。src/handlers/index.rs を読み込む。".to_string(),
        }),
    });

    let call_id = next_event_id();
    events.push(AgentEvent {
        id: call_id,
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::ToolCall(ToolCallPayload::FileRead {
            name: "Read".to_string(),
            arguments: FileReadArgs {
                file_path: Some("src/handlers/index.rs".to_string()),
                path: None,
                pattern: None,
                extra: serde_json::json!({}),
            },
            provider_call_id: Some("call_01".to_string()),
        }),
    });

    events.push(AgentEvent {
        id: next_event_id(),
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::ToolResult(ToolResultPayload {
            output: "// existing code... \n fn handle() -> Result<()> { ... }".to_string(),
            tool_call_id: call_id,
            is_error: false,
        }),
    });

    events.push(AgentEvent {
        id: next_event_id(),
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::Reasoning(ReasoningPayload {
            text: "コードを確認した。`std::fs::read_to_string` を使用している箇所で `unwrap()` が使われている。これを `context()` に置き換える。".to_string(),
        }),
    });

    let edit_id = next_event_id();
    events.push(AgentEvent {
        id: edit_id,
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::ToolCall(ToolCallPayload::FileEdit {
            name: "Edit".to_string(),
            arguments: FileEditArgs {
                file_path: "src/handlers/index.rs".to_string(),
                old_string: "unwrap()".to_string(),
                new_string: "context(\"failed to read file\")?".to_string(),
                replace_all: true,
            },
            provider_call_id: Some("call_02".to_string()),
        }),
    });

    events.push(AgentEvent {
        id: next_event_id(),
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::ToolResult(ToolResultPayload {
            output: "Applied 3 edits.".to_string(),
            tool_call_id: edit_id,
            is_error: false,
        }),
    });

    let exec_id = next_event_id();
    events.push(AgentEvent {
        id: exec_id,
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::ToolCall(ToolCallPayload::Execute {
            name: "Bash".to_string(),
            arguments: ExecuteArgs {
                command: Some("cargo check".to_string()),
                description: None,
                timeout: None,
                extra: serde_json::json!({}),
            },
            provider_call_id: Some("call_03".to_string()),
        }),
    });

    events.push(AgentEvent {
        id: next_event_id(),
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::ToolResult(ToolResultPayload {
            output: "error[E0425]: cannot find value `anyhow` in this scope".to_string(),
            tool_call_id: exec_id,
            is_error: true,
        }),
    });

    events.push(AgentEvent {
        id: next_event_id(),
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::Reasoning(ReasoningPayload {
            text: "あ、`use anyhow::Context;` のインポートを忘れていた。追加する必要がある。"
                .to_string(),
        }),
    });

    let edit2_id = next_event_id();
    events.push(AgentEvent {
        id: edit2_id,
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::ToolCall(ToolCallPayload::FileEdit {
            name: "Edit".to_string(),
            arguments: FileEditArgs {
                file_path: "src/handlers/index.rs".to_string(),
                old_string: "use std::fs;".to_string(),
                new_string: "use std::fs;\nuse anyhow::Context;".to_string(),
                replace_all: false,
            },
            provider_call_id: Some("call_04".to_string()),
        }),
    });

    events.push(AgentEvent {
        id: next_event_id(),
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::ToolResult(ToolResultPayload {
            output: "Applied 1 edit.".to_string(),
            tool_call_id: edit2_id,
            is_error: false,
        }),
    });

    let exec2_id = next_event_id();
    events.push(AgentEvent {
        id: exec2_id,
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::ToolCall(ToolCallPayload::Execute {
            name: "Bash".to_string(),
            arguments: ExecuteArgs {
                command: Some("cargo check".to_string()),
                description: None,
                timeout: None,
                extra: serde_json::json!({}),
            },
            provider_call_id: Some("call_05".to_string()),
        }),
    });

    events.push(AgentEvent {
        id: next_event_id(),
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::ToolResult(ToolResultPayload {
            output: "    Checking agtrace-cli v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s) in 2.3s".to_string(),
            tool_call_id: exec2_id,
            is_error: false,
        }),
    });

    events.push(AgentEvent {
        id: next_event_id(),
        session_id: session_uuid,
        parent_id: None,
        timestamp: tick(),
        stream_id: stream_id.clone(),
        metadata: None,
        payload: EventPayload::Message(MessagePayload {
            text: "修正完了しました。`anyhow::Context` を利用してエラー時のコンテキスト情報を付与するように変更し、インポートも追加しました。".to_string(),
        }),
    });

    events
}
