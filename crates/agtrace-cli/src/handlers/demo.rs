use crate::presentation::presenters::watch_tui::build_screen_view_model;
use crate::presentation::renderers::tui::{RendererSignal, TuiEvent, TuiRenderer};
use agtrace_engine::session::assemble_session;
use agtrace_runtime::SessionState;
use agtrace_types::{
    AgentEvent, EventPayload, ExecuteArgs, FileEditArgs, FileReadArgs, MessagePayload,
    ReasoningPayload, StreamId, ToolCallPayload, ToolResultPayload, TokenUsagePayload,
    UserPayload,
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
    state.context_window_limit = Some(180_000);
    state.model = Some("Demo Model".to_string());

    let mut events_buffer = VecDeque::new();
    let mut current_notification: Option<String> = None;
    let mut notification_ttl: usize = 0;

    let scenario = generate_scenario(&session_id, start_time);

    for (idx, event) in scenario.into_iter().enumerate() {
        match signal_rx.try_recv() {
            Ok(RendererSignal::Quit) => break,
            Err(mpsc::TryRecvError::Disconnected) => break,
            _ => {}
        }

        state.last_activity = event.timestamp;
        state.event_count += 1;

        // Apply state updates using engine logic (same as watch handler)
        let updates = agtrace_engine::extract_state_updates(&event);
        if updates.is_new_turn {
            state.turn_count += 1;
        }
        if let Some(usage) = updates.usage {
            state.current_usage = usage;
        }
        if let Some(model) = updates.model {
            state.model.get_or_insert(model);
        }
        if let Some(limit) = updates.context_window_limit {
            state.context_window_limit.get_or_insert(limit);
        }

        events_buffer.push_back(event);
        if events_buffer.len() > 100 {
            events_buffer.pop_front();
        }

        let max_context = state.context_window_limit.map(|x| x as u32);

        // Update notification with TTL (Time To Live)
        let new_notification = if idx == 0 {
            Some("DEMO MODE: Simulating active coding session...".to_string())
        } else if idx == 3 {
            Some("Large file detected, context usage spiked!".to_string())
        } else if idx == 15 {
            Some("Documentation loaded, context growing...".to_string())
        } else if idx == 30 {
            Some("Multiple files in context, approaching limits...".to_string())
        } else if idx == 50 {
            Some("⚠ Warning: Context window usage is getting high!".to_string())
        } else if idx == 70 {
            Some("🔴 Critical: Near maximum context window capacity!".to_string())
        } else {
            None
        };

        // Set new notification and reset TTL, or decrement TTL
        if let Some(notif) = new_notification {
            current_notification = Some(notif);
            notification_ttl = 15; // Display for 15 events
        } else if notification_ttl > 0 {
            notification_ttl -= 1;
            if notification_ttl == 0 {
                current_notification = None;
            }
        }

        let notification = current_notification.as_ref();

        let events_vec: Vec<_> = events_buffer.iter().cloned().collect();
        let assembled_session = assemble_session(&events_vec);

        let vm = build_screen_view_model(
            &state,
            &events_buffer,
            assembled_session.as_ref(),
            max_context,
            notification.map(|s| s.as_str()),
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
    turn_number: u32,
    step_input_tokens: i32,
    step_output_tokens: i32,
    total_context: i32,
    total_output_tokens: i32,
}

impl ScenarioBuilder {
    fn new(session_id: &str, start: DateTime<Utc>) -> Self {
        use std::str::FromStr;
        let session_uuid = uuid::Uuid::from_str(session_id).unwrap_or_else(|_| {
            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap()
        });

        Self {
            events: Vec::new(),
            session_uuid,
            stream_id: StreamId::Main,
            timestamp: start,
            event_counter: 0,
            call_counter: 0,
            turn_number: 0,
            step_input_tokens: 0,
            step_output_tokens: 0,
            total_context: 0,
            total_output_tokens: 0,
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

    /// Estimate tokens based on text length (roughly 4 chars per token)
    /// Returns a reasonable minimum to simulate realistic token usage
    fn estimate_tokens(text: &str) -> i32 {
        ((text.len() / 4).max(50) as i32).max(500)
    }

    /// Add tokens to current step accumulator
    fn add_step_tokens(&mut self, input: i32, output: i32) {
        self.step_input_tokens += input;
        self.step_output_tokens += output;
    }

    /// Emit accumulated tokens and reset step counter
    fn emit_step_tokens(&mut self) -> &mut Self {
        if self.step_input_tokens == 0 && self.step_output_tokens == 0 {
            return self;
        }

        let id = self.next_event_id();
        let ts = self.tick();

        // TokenUsage events report CUMULATIVE tokens
        // Both input and output are cumulative values

        // Each step processes: previous conversation + new action
        let new_action_tokens = self.step_input_tokens;
        let context_tokens = self.total_context;

        // Total input for this step = context + new action
        let step_total_input = context_tokens + new_action_tokens;

        // Update total context for next step (includes this step's input + output)
        self.total_context = step_total_input + self.step_output_tokens;

        // Update cumulative output tokens
        self.total_output_tokens += self.step_output_tokens;

        // Emit as CUMULATIVE values
        self.events.push(AgentEvent {
            id,
            session_id: self.session_uuid,
            parent_id: None,
            timestamp: ts,
            stream_id: self.stream_id.clone(),
            metadata: None,
            payload: EventPayload::TokenUsage(TokenUsagePayload {
                input_tokens: step_total_input,
                output_tokens: self.total_output_tokens,
                total_tokens: step_total_input + self.total_output_tokens,
                details: None,
            }),
        });

        self.step_input_tokens = 0;
        self.step_output_tokens = 0;
        self
    }

    fn user(&mut self, text: impl Into<String>) -> &mut Self {
        let id = self.next_event_id();
        let ts = self.tick();
        let text_str = text.into();

        self.turn_number += 1;

        self.events.push(AgentEvent {
            id,
            session_id: self.session_uuid,
            parent_id: None,
            timestamp: ts,
            stream_id: self.stream_id.clone(),
            metadata: None,
            payload: EventPayload::User(UserPayload { text: text_str }),
        });
        self
    }

    fn think(&mut self, text: impl Into<String>) -> &mut Self {
        let id = self.next_event_id();
        let ts = self.tick();
        let text_str = text.into();

        // Thinking adds moderate input cost (processing previous context)
        let tokens = Self::estimate_tokens(&text_str);
        self.add_step_tokens(tokens * 2, tokens / 2);

        self.events.push(AgentEvent {
            id,
            session_id: self.session_uuid,
            parent_id: None,
            timestamp: ts,
            stream_id: self.stream_id.clone(),
            metadata: None,
            payload: EventPayload::Reasoning(ReasoningPayload { text: text_str }),
        });
        self
    }

    fn read_file(&mut self, path: impl Into<String>, output: impl Into<String>) -> &mut Self {
        let call_id = self.next_event_id();
        let ts1 = self.tick();
        let provider_id = self.next_call_id();
        let path_str = path.into();
        let output_str = output.into();

        // File reads add significant input tokens (simulated file content)
        let file_tokens = Self::estimate_tokens(&output_str);
        // Large files contribute significantly to context
        // Multiply by 5-10x to simulate reading file + surrounding context
        let input_tokens = file_tokens * 8;
        self.add_step_tokens(input_tokens, 200);

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
                output: output_str,
                tool_call_id: call_id,
                is_error: false,
            }),
        });
        self
    }

    fn edit_file(
        &mut self,
        path: impl Into<String>,
        old: impl Into<String>,
        new: impl Into<String>,
        count: usize,
    ) -> &mut Self {
        let call_id = self.next_event_id();
        let ts1 = self.tick();
        let provider_id = self.next_call_id();
        let (path_str, old_str, new_str) = (path.into(), old.into(), new.into());

        // Edits require reading context + generating new code
        // Editing involves: reading file, understanding context, generating changes
        let edit_tokens = Self::estimate_tokens(&old_str) + Self::estimate_tokens(&new_str);
        let input_tokens = edit_tokens * 5 * count as i32; // Large context for edits
        let output_tokens = edit_tokens * 2 * count as i32; // Generated code
        self.add_step_tokens(input_tokens, output_tokens);

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
                output: format!(
                    "Applied {} edit{}.",
                    count,
                    if count == 1 { "" } else { "s" }
                ),
                tool_call_id: call_id,
                is_error: false,
            }),
        });
        self
    }

    fn bash(
        &mut self,
        cmd: impl Into<String>,
        is_error: bool,
        output: impl Into<String>,
    ) -> &mut Self {
        let call_id = self.next_event_id();
        let ts1 = self.tick();
        let provider_id = self.next_call_id();
        let cmd_str = cmd.into();
        let output_str = output.into();

        // Bash commands: command + output contribute to tokens
        let cmd_tokens = Self::estimate_tokens(&cmd_str);
        let output_tokens = Self::estimate_tokens(&output_str);
        self.add_step_tokens(cmd_tokens + output_tokens, 100);

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
                output: output_str,
                tool_call_id: call_id,
                is_error,
            }),
        });
        self
    }

    fn message(&mut self, text: impl Into<String>) -> &mut Self {
        let id = self.next_event_id();
        let ts = self.tick();
        let text_str = text.into();

        // Final message generation
        let message_tokens = Self::estimate_tokens(&text_str);
        self.add_step_tokens(200, message_tokens);

        self.events.push(AgentEvent {
            id,
            session_id: self.session_uuid,
            parent_id: None,
            timestamp: ts,
            stream_id: self.stream_id.clone(),
            metadata: None,
            payload: EventPayload::Message(MessagePayload { text: text_str }),
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
        .user("Improve error handling in src/handlers/index.rs. I want to add context using anyhow.");

    // AgentStep 1: Read the file
    builder
        .think("First, I need to check the current code. Reading src/handlers/index.rs.")
        .read_file("src/handlers/index.rs", "pub fn handle() -> Result<()> {\n    let data = std::fs::read_to_string(\"data.json\").unwrap();\n    Ok(())\n}")
        .emit_step_tokens(); // Large file read

    // AgentStep 2: First edit attempt
    builder
        .think("Found unwrap() being used. I'll replace it with context().")
        .edit_file("src/handlers/index.rs", "unwrap()", "context(\"failed to read file\")?", 3)
        .emit_step_tokens();

    // AgentStep 3: Check build and find error
    builder
        .bash("cargo check", true, "error[E0425]: cannot find value `anyhow` in this scope")
        .emit_step_tokens();

    // AgentStep 4: Add missing import
    builder
        .think("Forgot to import. Adding use anyhow::Context;")
        .edit_file("src/handlers/index.rs", "use std::fs;", "use std::fs;\nuse anyhow::Context;", 1)
        .emit_step_tokens();

    // AgentStep 5: Verify fix and complete
    builder
        .bash("cargo check", false, "    Checking agtrace-cli v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s) in 2.3s")
        .message("Completed the fix. Changed to use anyhow::Context to add context information on errors.")
        .emit_step_tokens();

    // Turn 2: Add tests
    builder
        .user("Please add tests as well");

    // AgentStep 1: Check for existing test files
    builder
        .think("First, let me check the test file structure")
        .bash("ls src/handlers/*.rs | grep test", true, "No such file")
        .emit_step_tokens();

    // AgentStep 2: Read test file
    builder
        .think("No test file exists. Creating one in tests/")
        .read_file("tests/integration_test.rs", "// Basic test structure...")
        .emit_step_tokens(); // Reading test file

    // AgentStep 3: Add first test
    builder
        .think("Adding test cases for index.rs")
        .edit_file("tests/integration_test.rs", "// Add tests here", "#[test]\nfn test_handle_success() {\n    let result = handle();\n    assert!(result.is_ok());\n}", 1)
        .bash("cargo test handle_success", false, "test test_handle_success ... ok\ntest result: ok. 1 passed; 0 failed")
        .emit_step_tokens();

    // AgentStep 4: Add error case test
    builder
        .think("Adding error case tests as well")
        .edit_file("tests/integration_test.rs", "}", "#[test]\nfn test_handle_missing_file() {\n    // Test error case\n}\n\n}", 1)
        .emit_step_tokens();

    // AgentStep 5: Run all tests and complete
    builder
        .bash("cargo test", false, "test result: ok. 2 passed; 0 failed")
        .message("Tests added. Covering both success and error cases.")
        .emit_step_tokens();

    // Turn 3: Performance optimization
    builder
        .user("Can we make it faster? I'm concerned about performance");

    // AgentStep 1: Benchmark current performance
    builder
        .think("First, let's benchmark to understand the current state")
        .bash("cargo build --release", false, "    Compiling agtrace-cli v0.1.0\n    Finished release [optimized] target(s) in 15.2s")
        .bash("hyperfine 'cargo run --release'", false, "Time (mean ± σ):     124.3 ms ±   2.1 ms")
        .emit_step_tokens();

    // AgentStep 2: Profile to find bottlenecks
    builder
        .think("Profiling to identify bottlenecks")
        .bash("cargo flamegraph", false, "Wrote flamegraph to flamegraph.svg")
        .emit_step_tokens();

    // AgentStep 3: Analyze bottleneck
    builder
        .think("File I/O is the main bottleneck. Using BufReader should improve performance")
        .read_file("src/handlers/index.rs", "let data = std::fs::read_to_string(\"data.json\").context(\"failed to read file\")?;")
        .emit_step_tokens(); // Reading code with context

    // AgentStep 4: Implement BufReader optimization
    builder
        .think("Introducing BufReader and caching")
        .edit_file("src/handlers/index.rs", "use std::fs;", "use std::fs::{self, File};\nuse std::io::{BufReader, BufRead};", 1)
        .edit_file("src/handlers/index.rs", "std::fs::read_to_string(\"data.json\")", "{\n    let file = File::open(\"data.json\")?;\n    let reader = BufReader::new(file);\n    reader.lines().collect::<Result<Vec<_>, _>>()?.join(\"\\n\")\n}", 1)
        .emit_step_tokens(); // Complex refactoring with context

    // AgentStep 5: Verify performance improvement
    builder
        .bash("cargo build --release", false, "    Finished release [optimized] target(s) in 3.8s")
        .bash("hyperfine 'cargo run --release'", false, "Time (mean ± σ):      89.1 ms ±   1.8 ms")
        .message("Achieved ~28% speedup using BufReader (124.3ms → 89.1ms).")
        .emit_step_tokens();

    // Turn 4: Add documentation
    builder
        .user("Please add documentation comments");

    // AgentStep 1: Check current documentation
    builder
        .think("Checking function documentation")
        .read_file("src/handlers/index.rs", "pub fn handle() -> Result<()> {")
        .emit_step_tokens(); // Large context from accumulated files

    // AgentStep 2: Add doc comments to main function
    builder
        .think("Adding doc comments")
        .edit_file("src/handlers/index.rs", "pub fn handle()", "/// Handles index operations with improved error context.\n///\n/// # Errors\n/// Returns error if file cannot be read or parsed.\n///\n/// # Examples\n/// ```\n/// let result = handle();\n/// assert!(result.is_ok());\n/// ```\npub fn handle()", 1)
        .bash("cargo doc --no-deps", false, "    Documenting agtrace-cli v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s) in 1.2s")
        .emit_step_tokens();

    // AgentStep 3: Add docs to other functions
    builder
        .think("Adding documentation to other public functions as well")
        .read_file("src/handlers/index.rs", "pub fn init_index()")
        .edit_file("src/handlers/index.rs", "pub fn init_index()", "/// Initializes the index database.\n/// \n/// # Errors\n/// Returns error if database cannot be created.\npub fn init_index()", 1)
        .emit_step_tokens();

    // AgentStep 4: Generate and open docs
    builder
        .bash("cargo doc --open", false, "Opening docs in browser...")
        .message("Documentation comments added. You can view them with cargo doc.")
        .emit_step_tokens();

    // Turn 5: Refactoring
    builder
        .user("I'm concerned about code duplication. Can you refactor it?");

    // AgentStep 1: Find duplication patterns
    builder
        .think("Looking for duplication patterns")
        .bash("grep -n 'context(\"failed' src/**/*.rs", false, "src/handlers/index.rs:15:    .context(\"failed to read file\")?;\nsrc/handlers/index.rs:42:    .context(\"failed to parse\")?;")
        .emit_step_tokens();

    // AgentStep 2: Create common error handling module
    builder
        .think("Creating a common error handling function")
        .edit_file("src/handlers/mod.rs", "pub mod index;", "pub mod index;\n\n/// Common error handling utilities\nmod errors {\n    pub fn read_file_error(path: &str) -> String {\n        format!(\"Failed to read file: {}\", path)\n    }\n}", 1)
        .emit_step_tokens();

    // AgentStep 3: Refactor index.rs to use common module
    builder
        .think("Updating to use it in index.rs")
        .edit_file("src/handlers/index.rs", "use std::fs", "use std::fs;\nuse super::errors", 1)
        .edit_file("src/handlers/index.rs", ".context(\"failed to read file\")?", ".context(errors::read_file_error(\"data.json\"))?", 1)
        .emit_step_tokens();

    // AgentStep 4: Verify refactoring
    builder
        .bash("cargo check", false, "    Checking agtrace-cli v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s) in 1.1s")
        .bash("cargo clippy", false, "    Checking agtrace-cli v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s) in 1.3s")
        .message("Refactored error handling into common functions.")
        .emit_step_tokens();

    // Turn 6: Add logging
    builder
        .user("I want to add logging to make debugging easier");

    // AgentStep 1: Add tracing dependencies
    builder
        .think("The tracing crate looks like a good choice")
        .bash("cargo add tracing tracing-subscriber", false, "    Updating crates.io index\n      Adding tracing v0.1.40\n      Adding tracing-subscriber v0.3.18")
        .emit_step_tokens();

    // AgentStep 2: Initialize tracing
    builder
        .think("Adding initialization code")
        .edit_file("src/main.rs", "fn main()", "fn main() {\n    tracing_subscriber::fmt::init();\n", 1)
        .emit_step_tokens();

    // AgentStep 3: Add log statements to handlers
    builder
        .think("Adding logs to handler functions")
        .edit_file("src/handlers/index.rs", "use super::errors", "use super::errors;\nuse tracing::{info, debug, error};", 1)
        .edit_file("src/handlers/index.rs", "pub fn handle() -> Result<()> {", "pub fn handle() -> Result<()> {\n    info!(\"Starting handle operation\");", 1)
        .edit_file("src/handlers/index.rs", "let reader = BufReader::new(file);", "let reader = BufReader::new(file);\n    debug!(\"Reading file with BufReader\");", 1)
        .emit_step_tokens(); // Multiple edits with large context

    // AgentStep 4: Test logging output
    builder
        .bash("cargo run", false, "2024-01-15T10:23:45.123Z  INFO agtrace_cli::handlers::index: Starting handle operation\n2024-01-15T10:23:45.125Z DEBUG agtrace_cli::handlers::index: Reading file with BufReader")
        .message("Added logging using tracing. Can be controlled with the RUST_LOG environment variable.")
        .emit_step_tokens();

    // Turn 7: Add configuration
    builder
        .user("Please make it possible to customize behavior with a configuration file");

    // AgentStep 1: Add dependencies
    builder
        .think("Adding functionality to read config.toml")
        .bash("cargo add serde toml", false, "    Updating crates.io index\n      Adding serde v1.0\n      Adding toml v0.8")
        .emit_step_tokens();

    // AgentStep 2: Define config struct
    builder
        .think("Defining configuration struct")
        .bash("touch src/config.rs", false, "")
        .edit_file("src/config.rs", "", "use serde::Deserialize;\n\n#[derive(Debug, Deserialize)]\npub struct Config {\n    pub buffer_size: usize,\n    pub enable_cache: bool,\n}", 1)
        .edit_file("src/lib.rs", "pub mod handlers;", "pub mod handlers;\npub mod config;", 1)
        .emit_step_tokens();

    // AgentStep 3: Create config file and integrate
    builder
        .think("Loading from config.toml")
        .bash("echo '[settings]\\nbuffer_size = 8192\\nenable_cache = true' > config.toml", false, "")
        .edit_file("src/handlers/index.rs", "pub fn handle()", "pub fn handle(config: &crate::config::Config)", 1)
        .emit_step_tokens(); // Near context limit

    // AgentStep 4: Build and verify
    builder
        .bash("cargo build", false, "    Compiling agtrace-cli v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s) in 2.1s")
        .message("Added support for configuration file (config.toml).")
        .emit_step_tokens();

    builder.build()
}
