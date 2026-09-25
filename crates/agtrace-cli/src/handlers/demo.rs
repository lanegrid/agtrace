//! `agtrace demo`: replay a synthetic multi-agent workspace and watch it live.
//!
//! The scenario is written as raw provider logs (Claude Code ≥ 2.1.24x transcripts,
//! team config, session registry and subagent meta files; Codex ≥ 0.153 paginated
//! rollouts) into a temporary directory, line by line at real-time pace. The real
//! workspace watcher and TUI run against that directory, so the demo doubles as an
//! end-to-end smoke test of discovery → decoding → graph → TUI.
//!
//! Every value is synthetic (ids, paths under `/work/demo-project`, encrypted
//! bodies `gAAAA_SYNTHETIC_…`).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use agtrace_sdk::watch::{LiveWorkspace, WatchRoots, WatchScope, WatcherOptions};
use anyhow::{Context, Result};
use chrono::{DateTime, Local, Offset, Utc};
use is_terminal::IsTerminal;
use serde_json::{Value, json};

use crate::handlers::watch::{LiveSource, run};
use crate::presentation::view_models::watch::UiState;

/// Project cwd of every demo agent.
pub const PROJECT: &str = "/work/demo-project";
const LEAD: &str = "d0000000-0000-4000-8000-000000000001";
const TEAMMATE: &str = "d0000000-0000-4000-8000-000000000002";
const SUBAGENT: &str = "ad000000000000001";
const TEAM: &str = "demo-team";
const CODEX_ROOT: &str = "01900000-0000-7000-8000-0000000000d1";
const CODEX_CHILD: &str = "01900000-0000-7000-8000-0000000000d2";
const CLAUDE_VERSION: &str = "2.1.281";
const CODEX_VERSION: &str = "0.155.0";

pub fn handle(speed: String) -> Result<()> {
    if !std::io::stdout().is_terminal() {
        anyhow::bail!("demo requires a TTY (interactive terminal)");
    }
    let speed = match speed.as_str() {
        "slow" => 0.5,
        "fast" => 4.0,
        _ => 1.0,
    };

    let dir = TempWorkspace::create()?;
    let start = Utc::now();
    let steps = scenario(start, speed, std::process::id(), Local::now().date_naive());
    let live = LiveWorkspace::watch_roots(
        WatchScope::Project {
            root: PathBuf::from(PROJECT),
            since: WatchScope::DEFAULT_SINCE,
        },
        dir.roots(),
        WatcherOptions::default(),
        Arc::new(agtrace_sdk::utils::builtin_model_catalog()),
    )?;

    let stop = Arc::new(AtomicBool::new(false));
    let writer = {
        let root = dir.path.clone();
        let stop = stop.clone();
        std::thread::spawn(move || replay(&root, &steps, &stop))
    };

    let mut ui = UiState::new("demo · project demo-project", Local::now().offset().fix());
    ui.auto_select = true;
    let result = run(&LiveSource(live), ui);

    stop.store(true, Ordering::Release);
    let replayed = writer
        .join()
        .map_err(|_| anyhow::anyhow!("demo writer panicked"))?;
    result.and(replayed)
}

// ============================================================================
// Temporary workspace + replay
// ============================================================================

/// Provider homes under a fresh temporary directory, removed on drop.
pub struct TempWorkspace {
    pub path: PathBuf,
}

impl TempWorkspace {
    pub fn create() -> Result<Self> {
        let nanos = Utc::now().timestamp_nanos_opt().unwrap_or_default();
        let path =
            std::env::temp_dir().join(format!("agtrace-demo-{}-{nanos:x}", std::process::id()));
        fs::create_dir_all(&path)
            .with_context(|| format!("failed to create {}", path.display()))?;
        Ok(Self { path })
    }

    /// Watcher roots of the replayed workspace.
    pub fn roots(&self) -> WatchRoots {
        WatchRoots {
            claude_home: Some(self.path.join("claude")),
            claude_projects: Some(self.path.join("claude/projects")),
            codex_sessions: Some(self.path.join("codex/sessions")),
        }
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// One file operation of the scenario.
#[derive(Debug, Clone)]
pub struct Step {
    /// Wall-clock offset from the scenario start.
    pub at: Duration,
    /// Path relative to the workspace root (`claude/...`, `codex/...`).
    pub path: PathBuf,
    pub op: Op,
}

#[derive(Debug, Clone)]
pub enum Op {
    /// Append one JSONL line.
    Append(String),
    /// Replace the whole file (sidecars).
    Write(String),
}

/// Apply every step to `root`. Waits for each step's time unless `stop` is set
/// (then returns early).
pub fn replay(root: &Path, steps: &[Step], stop: &AtomicBool) -> Result<()> {
    let started = Instant::now();
    for step in steps {
        loop {
            if stop.load(Ordering::Acquire) {
                return Ok(());
            }
            let elapsed = started.elapsed();
            if elapsed >= step.at {
                break;
            }
            std::thread::sleep((step.at - elapsed).min(Duration::from_millis(100)));
        }
        apply(root, step)?;
    }
    Ok(())
}

/// Apply one step (creates parent directories).
pub fn apply(root: &Path, step: &Step) -> Result<()> {
    let path = root.join(&step.path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    match &step.op {
        Op::Append(line) => {
            let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
            writeln!(file, "{line}")?;
        }
        Op::Write(content) => {
            // Write-then-rename so readers never see a half-written sidecar.
            let tmp = path.with_extension("tmp~");
            fs::write(&tmp, content)?;
            fs::rename(&tmp, &path)?;
        }
    }
    Ok(())
}

// ============================================================================
// Scenario
// ============================================================================

/// Scenario builder: turns scenario seconds into wall-clock offsets / timestamps.
struct Script {
    start: DateTime<Utc>,
    speed: f64,
    steps: Vec<Step>,
    uuid: u64,
}

impl Script {
    fn offset(&self, secs: f64) -> Duration {
        Duration::from_secs_f64((secs / self.speed).max(0.0))
    }

    fn ts(&self, secs: f64) -> String {
        let at = self.start + chrono::Duration::from_std(self.offset(secs)).unwrap_or_default();
        at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }

    fn millis(&self, secs: f64) -> i64 {
        let at = self.start + chrono::Duration::from_std(self.offset(secs)).unwrap_or_default();
        at.timestamp_millis()
    }

    fn next_uuid(&mut self) -> String {
        self.uuid += 1;
        format!("d0000000-0000-4000-9000-{:012x}", self.uuid)
    }

    fn append(&mut self, secs: f64, path: &str, line: Value) {
        self.steps.push(Step {
            at: self.offset(secs),
            path: PathBuf::from(path),
            op: Op::Append(line.to_string()),
        });
    }

    fn write(&mut self, secs: f64, path: &str, content: Value) {
        self.steps.push(Step {
            at: self.offset(secs),
            path: PathBuf::from(path),
            op: Op::Write(serde_json::to_string_pretty(&content).unwrap_or_default()),
        });
    }
}

/// A Claude Code transcript being written.
struct ClaudeLog {
    path: String,
    session: &'static str,
    /// `(agentId)` for subagent transcripts (`isSidechain`).
    agent_id: Option<&'static str>,
    /// `(teamName, agentName)` envelope of a teammate.
    member: Option<(&'static str, &'static str)>,
    parent: Option<String>,
    msg: u32,
    /// Context tokens of the next assistant record (grows over the scenario).
    context: u64,
}

impl ClaudeLog {
    fn envelope(&mut self, s: &mut Script, secs: f64, kind: &str) -> Value {
        let uuid = s.next_uuid();
        let mut v = json!({
            "parentUuid": self.parent,
            "isSidechain": self.agent_id.is_some(),
            "userType": "external",
            "cwd": PROJECT,
            "sessionId": self.session,
            "version": CLAUDE_VERSION,
            "gitBranch": "main",
            "type": kind,
            "uuid": uuid,
            "timestamp": s.ts(secs),
        });
        if let Some(aid) = self.agent_id {
            v["agentId"] = json!(aid);
        } else {
            v["entrypoint"] = json!("cli");
        }
        if let Some((team, name)) = self.member {
            v["teamName"] = json!(team);
            v["agentName"] = json!(name);
        }
        self.parent = Some(uuid);
        v
    }

    fn state(&self, s: &mut Script, secs: f64, mut v: Value) {
        v["sessionId"] = json!(self.session);
        s.append(secs, &self.path, v);
    }

    fn user(&mut self, s: &mut Script, secs: f64, content: Value, extra: Value) {
        let mut v = self.envelope(s, secs, "user");
        v["message"] = json!({"role": "user", "content": content});
        merge(&mut v, extra);
        s.append(secs, &self.path, v);
    }

    fn model(&mut self, s: &mut Script, secs: f64, id: &str, name: &str) {
        let mut v = self.envelope(s, secs, "attachment");
        v["attachment"] = json!({
            "type": "model",
            "identity": {"modelId": id, "marketingName": name, "knowledgeCutoff": "June 2026"},
            "text": format!("You are {name}."),
        });
        s.append(secs, &self.path, v);
    }

    /// One assistant record (one content block) with usage; `grow` adds context.
    fn assistant(
        &mut self,
        s: &mut Script,
        secs: f64,
        block: Value,
        stop: Option<&str>,
        grow: u64,
    ) {
        self.msg += 1;
        self.context += grow;
        let prefix = self.agent_id.unwrap_or(self.session);
        let id = format!("msg_demo_{}_{}", &prefix[..8.min(prefix.len())], self.msg);
        let mut v = self.envelope(s, secs, "assistant");
        v["requestId"] = json!(format!("req_{id}"));
        v["message"] = json!({
            "model": "claude-opus-5-5",
            "id": id,
            "type": "message",
            "role": "assistant",
            "content": [block],
            "stop_reason": stop,
            "stop_sequence": null,
            "usage": {
                "input_tokens": 6,
                "cache_creation_input_tokens": 400,
                "cache_read_input_tokens": self.context.saturating_sub(406),
                "output_tokens": 120,
                "service_tier": "standard",
            },
        });
        s.append(secs, &self.path, v);
    }

    fn text(&mut self, s: &mut Script, secs: f64, text: &str, stop: Option<&str>, grow: u64) {
        self.assistant(s, secs, json!({"type": "text", "text": text}), stop, grow);
    }

    fn tool_use(
        &mut self,
        s: &mut Script,
        secs: f64,
        id: &str,
        name: &str,
        input: Value,
        grow: u64,
    ) {
        let block = json!({"type": "tool_use", "id": id, "name": name, "input": input, "caller": {"type": "direct"}});
        self.assistant(s, secs, block, Some("tool_use"), grow);
    }

    fn tool_result(&mut self, s: &mut Script, secs: f64, id: &str, output: &str, result: Value) {
        let content = json!([{"type": "tool_result", "tool_use_id": id, "content": output, "is_error": false}]);
        self.user(s, secs, content, json!({"toolUseResult": result}));
    }

    fn system(&mut self, s: &mut Script, secs: f64, subtype: &str, extra: Value) {
        let mut v = self.envelope(s, secs, "system");
        v["subtype"] = json!(subtype);
        v["isMeta"] = json!(false);
        merge(&mut v, extra);
        s.append(secs, &self.path, v);
    }
}

/// A Codex rollout being written.
struct CodexLog {
    path: String,
    thread: &'static str,
    ordinal: u64,
    turn: &'static str,
    context: u64,
}

impl CodexLog {
    fn record(&mut self, s: &mut Script, secs: f64, kind: &str, payload: Value) {
        let v = json!({"timestamp": s.ts(secs), "type": kind, "ordinal": self.ordinal, "payload": payload});
        self.ordinal += 1;
        s.append(secs, &self.path, v);
    }

    fn event(&mut self, s: &mut Script, secs: f64, payload: Value) {
        self.record(s, secs, "event_msg", payload);
    }

    fn item(&mut self, s: &mut Script, secs: f64, item: Value) {
        let payload = json!({
            "type": "item_completed",
            "thread_id": self.thread,
            "turn_id": self.turn,
            "item": item,
        });
        self.event(s, secs, payload);
    }

    fn response(&mut self, s: &mut Script, secs: f64, mut payload: Value) {
        payload["internal_chat_message_metadata_passthrough"] = json!({"turn_id": self.turn});
        self.record(s, secs, "response_item", payload);
    }

    fn usage(&mut self, s: &mut Script, secs: f64, grow: u64) {
        self.context += grow;
        let usage = json!({
            "input_tokens": self.context,
            "cached_input_tokens": self.context * 9 / 10,
            "cache_write_input_tokens": 0,
            "output_tokens": 150,
            "reasoning_output_tokens": 40,
            "total_tokens": self.context + 150,
        });
        let payload = json!({
            "thread_id": self.thread,
            "session_id": CODEX_ROOT,
            "turn_id": self.turn,
            "root_turn_id": "turn-d1",
            "response_id": format!("resp_demo_{}", self.ordinal),
            "usage": usage,
            "turn_token_usage": usage,
            "thread_token_usage": usage,
        });
        self.record(s, secs, "token_usage_record", payload);
    }

    /// `exec` tool call running `cmd`; the result `(stdout, exit code)` lands `took`
    /// seconds later.
    fn exec(
        &mut self,
        s: &mut Script,
        secs: f64,
        call: &str,
        cmd: &str,
        took: f64,
        (out, exit): (&str, i32),
    ) {
        let input = format!(
            "const r = await tools.exec_command({}); text(r.output);",
            json!({"cmd": cmd, "workdir": PROJECT})
        );
        self.response(
            s,
            secs,
            json!({"type": "custom_tool_call", "id": format!("ctc_{call}"), "call_id": call, "name": "exec", "input": input, "status": "completed"}),
        );
        self.usage(s, secs + 0.2, 6_000);
        let status = if exit == 0 { "completed" } else { "failed" };
        self.item(
            s,
            secs + took,
            json!({
                "type": "CommandExecution", "id": format!("exec-{call}"),
                "command": ["/bin/zsh", "-lc", cmd], "cwd": format!("file://{PROJECT}"),
                "process_id": "1", "source": "unified_exec_startup", "status": status,
                "exit_code": exit, "stdout": out, "stderr": "", "aggregated_output": out,
                "formatted_output": out, "duration": {"secs": took as u64, "nanos": 0},
                "parsed_cmd": [{"type": "unknown", "cmd": cmd}],
            }),
        );
        self.response(
            s,
            secs + took + 0.1,
            json!({"type": "custom_tool_call_output", "id": format!("ctco_{call}"), "call_id": call,
                   "output": format!("Script completed\nWall time {took:.1} seconds\nOutput:\n{out}")}),
        );
    }

    fn function_call(&mut self, s: &mut Script, secs: f64, call: &str, name: &str, args: Value) {
        self.response(
            s,
            secs,
            json!({"type": "function_call", "id": format!("fc_{call}"), "call_id": call, "name": name,
                   "arguments": args.to_string(), "namespace": "collaboration"}),
        );
    }

    fn function_output(&mut self, s: &mut Script, secs: f64, call: &str, output: &str) {
        self.response(
            s,
            secs,
            json!({"type": "function_call_output", "id": format!("fco_{call}"), "call_id": call, "output": output}),
        );
    }

    fn session_meta(&mut self, s: &mut Script, secs: f64, source: Value, extra: Value) {
        let mut payload = json!({
            "id": self.thread,
            "timestamp": s.ts(0.0),
            "session_id": CODEX_ROOT,
            "cwd": PROJECT,
            "originator": "codex-tui",
            "cli_version": CODEX_VERSION,
            "source": source,
            "model_provider": "openai",
            "history_mode": "paginated",
        });
        merge(&mut payload, extra);
        self.record(s, secs, "session_meta", payload);
    }

    fn turn_start(&mut self, s: &mut Script, secs: f64) {
        self.event(
            s,
            secs,
            json!({"type": "task_started", "turn_id": self.turn, "started_at": s.millis(secs) / 1000,
                   "model_context_window": 258_400, "collaboration_mode_kind": "default"}),
        );
        self.record(
            s,
            secs + 0.1,
            "turn_context",
            json!({"turn_id": self.turn, "root_turn_id": "turn-d1", "model": "gpt-5.6-sol", "effort": "medium",
                   "cwd": PROJECT, "multi_agent_version": "v2"}),
        );
    }

    fn final_answer(&mut self, s: &mut Script, secs: f64, text: &str, started: f64) {
        self.response(
            s,
            secs,
            json!({"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": text}],
                   "id": format!("msg_demo_final_{}", self.ordinal), "phase": "final_answer"}),
        );
        self.event(
            s,
            secs + 0.2,
            json!({"type": "task_complete", "turn_id": self.turn, "started_at": s.millis(started) / 1000,
                   "completed_at": s.millis(secs) / 1000, "duration_ms": ((secs - started) * 1000.0) as u64,
                   "last_agent_message": text}),
        );
    }
}

fn merge(target: &mut Value, extra: Value) {
    if let (Some(t), Value::Object(e)) = (target.as_object_mut(), extra) {
        t.extend(e);
    }
}

/// The demo scenario (~70 scenario seconds; `speed` scales wall-clock time).
///
/// - Claude Code lead: spawns teammate `audit-A` (team `demo-team`) and a background
///   subagent, runs tools, receives the teammate's report and the subagent's
///   hand-back, fixes the parser, compacts, and stops the teammate.
/// - Codex root: runs commands, spawns `/root/judge`, waits for its FINAL_ANSWER.
pub fn scenario(start: DateTime<Utc>, speed: f64, pid: u32, today: chrono::NaiveDate) -> Vec<Step> {
    let mut s = Script {
        start,
        speed: speed.max(0.01),
        steps: Vec::new(),
        uuid: 0,
    };
    let project_dir = "claude/projects/-work-demo-project";
    let mut lead = ClaudeLog {
        path: format!("{project_dir}/{LEAD}.jsonl"),
        session: LEAD,
        agent_id: None,
        member: None,
        parent: None,
        msg: 0,
        context: 38_000,
    };
    let mut mate = ClaudeLog {
        path: format!("{project_dir}/{TEAMMATE}.jsonl"),
        session: TEAMMATE,
        agent_id: None,
        member: Some((TEAM, "audit-A")),
        parent: None,
        msg: 0,
        context: 21_000,
    };
    let mut sub = ClaudeLog {
        path: format!("{project_dir}/{LEAD}/subagents/agent-{SUBAGENT}.jsonl"),
        session: LEAD,
        agent_id: Some(SUBAGENT),
        member: None,
        parent: None,
        msg: 0,
        context: 6_000,
    };
    let day = today.format("%Y/%m/%d").to_string();
    let stamp = start.with_timezone(&Local).format("%Y-%m-%dT%H-%M-%S");
    let mut root = CodexLog {
        path: format!("codex/sessions/{day}/rollout-{stamp}-{CODEX_ROOT}.jsonl"),
        thread: CODEX_ROOT,
        ordinal: 0,
        turn: "turn-d1",
        context: 14_000,
    };
    let mut judge = CodexLog {
        path: format!("codex/sessions/{day}/rollout-{stamp}-{CODEX_CHILD}.jsonl"),
        thread: CODEX_CHILD,
        ordinal: 0,
        turn: "turn-d2",
        context: 9_000,
    };
    let registry = format!("claude/sessions/{pid}.json");
    let registry_entry = |s: &Script, status: &str, secs: f64| {
        json!({
            "pid": pid, "sessionId": LEAD, "cwd": PROJECT, "startedAt": s.millis(0.0),
            "version": CLAUDE_VERSION, "kind": "interactive", "entrypoint": "cli",
            "name": "demo-lead", "status": status, "updatedAt": s.millis(secs),
            "statusUpdatedAt": s.millis(secs),
        })
    };

    // --- Claude lead: prompt, spawns ------------------------------------------------
    lead.state(
        &mut s,
        0.0,
        json!({"type": "permission-mode", "permissionMode": "auto"}),
    );
    lead.state(
        &mut s,
        0.0,
        json!({"type": "ai-title", "aiTitle": "Audit and fix the parser"}),
    );
    lead.model(&mut s, 0.0, "claude-opus-5-5", "Opus 5.5 (1M context)");
    let entry = registry_entry(&s, "busy", 0.0);
    s.write(0.0, &registry, entry);
    lead.user(
        &mut s,
        0.5,
        json!("Audit the parser with a small team, then fix what you find."),
        json!({"origin": {"kind": "human"}, "promptSource": "typed"}),
    );
    lead.text(
        &mut s,
        2.0,
        "I'll bring in a teammate and a background agent.",
        None,
        1_200,
    );
    lead.tool_use(
        &mut s,
        3.0,
        "toolu_demo_team",
        "Agent",
        json!({"description": "Audit parser", "prompt": "Review the parser module.",
               "subagent_type": "general-purpose", "model": "opus", "name": "audit-A"}),
        800,
    );
    let team = json!({
        "name": TEAM, "createdAt": s.millis(3.5), "leadAgentId": format!("team-lead@{TEAM}"),
        "leadSessionId": LEAD,
        "members": [
            {"agentId": format!("team-lead@{TEAM}"), "name": "team-lead", "agentType": "team-lead",
             "joinedAt": s.millis(3.5), "cwd": PROJECT, "backendType": "in-process"},
            {"agentId": format!("audit-A@{TEAM}"), "name": "audit-A", "agentType": "general-purpose",
             "model": "claude-opus-5-5[1m]", "color": "blue", "joinedAt": s.millis(3.5), "cwd": PROJECT,
             "backendType": "in-process", "isActive": true},
        ],
    });
    s.write(3.5, &format!("claude/teams/{TEAM}/config.json"), team);
    lead.tool_result(
        &mut s,
        4.0,
        "toolu_demo_team",
        &format!("Spawned successfully.\nagent_id: audit-A@{TEAM}"),
        json!({"status": "teammate_spawned", "teammate_id": format!("audit-A@{TEAM}"),
               "agent_id": format!("audit-A@{TEAM}"), "agent_type": "general-purpose", "model": "opus",
               "name": "audit-A", "color": "blue", "team_name": TEAM, "prompt": "Review the parser module."}),
    );
    lead.tool_use(
        &mut s,
        5.0,
        "toolu_demo_async",
        "Agent",
        json!({"description": "Count source files", "prompt": "Count the files in src.",
               "subagent_type": "general-purpose", "model": "opus", "run_in_background": true}),
        700,
    );
    let meta = json!({"agentType": "general-purpose", "description": "Count source files",
                      "toolUseId": "toolu_demo_async", "spawnDepth": 1, "model": "opus",
                      "requestShape": "background"});
    s.write(
        5.5,
        &format!("{project_dir}/{LEAD}/subagents/agent-{SUBAGENT}.meta.json"),
        meta,
    );
    lead.tool_result(
        &mut s,
        6.0,
        "toolu_demo_async",
        &format!("Async agent launched successfully.\nagentId: {SUBAGENT}"),
        json!({"isAsync": true, "status": "async_launched", "agentId": SUBAGENT,
               "description": "Count source files", "resolvedModel": "claude-opus-5-5[1m]",
               "prompt": "Count the files in src."}),
    );

    // --- Teammate audit-A ----------------------------------------------------------
    mate.state(
        &mut s,
        4.2,
        json!({"type": "agent-setting", "agentSetting": "general-purpose"}),
    );
    mate.user(
        &mut s,
        4.5,
        json!("<teammate-message teammate_id=\"team-lead\">\nReview the parser module.\n</teammate-message>"),
        json!({}),
    );
    mate.model(&mut s, 4.6, "claude-opus-5-5", "Opus 5.5 (1M context)");
    mate.tool_use(
        &mut s,
        7.0,
        "toolu_demo_t_read",
        "Read",
        json!({"file_path": format!("{PROJECT}/src/parser.rs")}),
        4_000,
    );
    mate.tool_result(
        &mut s,
        8.0,
        "toolu_demo_t_read",
        "fn parse() { /* ... */ }",
        json!({}),
    );
    mate.tool_use(
        &mut s,
        10.0,
        "toolu_demo_t_test",
        "Bash",
        json!({"command": "cargo test -p parser", "description": "Run parser tests"}),
        6_000,
    );
    mate.tool_result(
        &mut s,
        22.0,
        "toolu_demo_t_test",
        "test result: FAILED. 3 failed",
        json!({"stdout": "test result: FAILED. 3 failed", "stderr": "", "interrupted": false}),
    );
    mate.tool_use(
        &mut s,
        25.0,
        "toolu_demo_t_send",
        "SendMessage",
        json!({"to": "team-lead", "summary": "found 3 bugs", "message": "Done: 3 parser bugs (empty input, trailing comma, unicode escapes)."}),
        2_000,
    );
    mate.tool_result(
        &mut s,
        26.0,
        "toolu_demo_t_send",
        "Message sent to team-lead's inbox",
        json!({"success": true, "message": "Message sent to team-lead's inbox",
               "routing": {"sender": "audit-A", "target": "@team-lead", "summary": "found 3 bugs",
                           "content": "Done: 3 parser bugs (empty input, trailing comma, unicode escapes)."}}),
    );
    mate.system(
        &mut s,
        27.0,
        "turn_duration",
        json!({"durationMs": 22_500, "messageCount": 8}),
    );

    // --- Background subagent -------------------------------------------------------
    sub.user(&mut s, 6.5, json!("Count the files in src."), json!({}));
    sub.model(&mut s, 6.6, "claude-opus-5-5", "Opus 5.5 (1M context)");
    sub.tool_use(
        &mut s,
        8.0,
        "toolu_demo_s_ls",
        "Bash",
        json!({"command": "ls src | wc -l", "description": "Count files"}),
        1_500,
    );
    sub.tool_result(
        &mut s,
        11.0,
        "toolu_demo_s_ls",
        "7",
        json!({"stdout": "7", "stderr": "", "interrupted": false}),
    );
    sub.tool_use(
        &mut s,
        13.0,
        "toolu_demo_s_hb",
        "SubagentHandback",
        json!({"message": "There are 7 files in src."}),
        500,
    );
    sub.tool_result(
        &mut s,
        14.0,
        "toolu_demo_s_hb",
        "Report delivered to your caller.",
        json!({"success": true, "message": "Report delivered to your caller."}),
    );

    // --- Lead: work, reports, fix, compaction ------------------------------------------
    lead.tool_use(
        &mut s,
        8.0,
        "toolu_demo_clippy",
        "Bash",
        json!({"command": "cargo clippy --all-targets", "description": "Lint"}),
        3_000,
    );
    lead.tool_result(
        &mut s,
        14.5,
        "toolu_demo_clippy",
        "warning: 2 warnings emitted",
        json!({"stdout": "warning: 2 warnings emitted", "stderr": "", "interrupted": false}),
    );
    let handback = format!(
        "<agent-message from=\"{SUBAGENT}\">[Subagent hand-back] There are 7 files in src.</agent-message>"
    );
    let mut queued = lead.envelope(&mut s, 15.0, "attachment");
    queued["attachment"] = json!({
        "type": "queued_command", "prompt": handback, "commandMode": "prompt",
        "origin": {"kind": "peer", "from": SUBAGENT, "senderTaskId": SUBAGENT,
                   "body": "[Subagent hand-back] There are 7 files in src.", "handback": true},
    });
    s.append(15.0, &lead.path, queued);
    lead.tool_use(
        &mut s,
        17.0,
        "toolu_demo_read",
        "Read",
        json!({"file_path": format!("{PROJECT}/src/parser.rs")}),
        40_000,
    );
    lead.tool_result(
        &mut s,
        18.0,
        "toolu_demo_read",
        "fn parse() { /* ... */ }",
        json!({}),
    );
    lead.text(
        &mut s,
        20.0,
        "Waiting for audit-A's report.",
        Some("end_turn"),
        2_000,
    );
    lead.system(
        &mut s,
        20.5,
        "turn_duration",
        json!({"durationMs": 20_000, "messageCount": 16}),
    );
    let entry = registry_entry(&s, "idle", 20.5);
    s.write(20.5, &registry, entry);

    let teammate_msg = format!(
        "Another Claude session sent a message:\n<teammate-message teammate_id=\"audit-A\" color=\"blue\" summary=\"found 3 bugs\">\nDone: 3 parser bugs (empty input, trailing comma, unicode escapes).\n</teammate-message>\n\n<teammate-message teammate_id=\"audit-A\" color=\"blue\">{}</teammate-message>",
        json!({"type": "idle_notification", "from": "audit-A", "timestamp": s.ts(27.0), "idleReason": "available"})
    );
    let entry = registry_entry(&s, "busy", 28.0);
    s.write(28.0, &registry, entry);
    lead.user(
        &mut s,
        28.0,
        json!(teammate_msg),
        json!({"origin": {"kind": "human"}}),
    );
    lead.tool_use(
        &mut s,
        30.0,
        "toolu_demo_edit",
        "Edit",
        json!({"file_path": format!("{PROJECT}/src/parser.rs"), "old_string": "fn parse()", "new_string": "fn parse(input: &str)"}),
        60_000,
    );
    lead.tool_result(
        &mut s,
        31.0,
        "toolu_demo_edit",
        "The file has been updated.",
        json!({}),
    );
    lead.tool_use(
        &mut s,
        33.0,
        "toolu_demo_test",
        "Bash",
        json!({"command": "mise run test", "description": "Run all tests"}),
        90_000,
    );
    lead.tool_result(
        &mut s,
        45.0,
        "toolu_demo_test",
        "test result: ok. 142 passed",
        json!({"stdout": "test result: ok. 142 passed", "stderr": "", "interrupted": false}),
    );
    lead.tool_use(
        &mut s,
        47.0,
        "toolu_demo_send",
        "SendMessage",
        json!({"to": "audit-A", "summary": "thanks", "message": "Fixed all three, tests pass. Thanks!"}),
        150_000,
    );
    lead.tool_result(
        &mut s,
        48.0,
        "toolu_demo_send",
        "Message sent to audit-A's inbox",
        json!({"success": true, "message": "Message sent to audit-A's inbox",
               "routing": {"sender": "team-lead", "target": "@audit-A", "summary": "thanks",
                           "content": "Fixed all three, tests pass. Thanks!"}}),
    );
    let pre = lead.context;
    lead.system(
        &mut s,
        52.0,
        "compact_boundary",
        json!({"content": "Conversation compacted", "level": "info",
               "compactMetadata": {"trigger": "auto", "preTokens": pre, "postTokens": 42_000, "durationMs": 3_000}}),
    );
    lead.context = 42_000;
    lead.tool_use(
        &mut s,
        55.0,
        "toolu_demo_stop",
        "TaskStop",
        json!({"task_id": "audit-A"}),
        1_000,
    );
    lead.tool_result(
        &mut s,
        56.0,
        "toolu_demo_stop",
        "Successfully stopped task: audit-A",
        json!({"message": "Successfully stopped task: audit-A", "task_id": "t_demo_1",
               "task_type": "in_process_teammate", "command": "audit-A"}),
    );
    lead.text(
        &mut s,
        58.0,
        "The parser is fixed: 3 bugs, all tests pass.",
        Some("end_turn"),
        800,
    );
    lead.system(
        &mut s,
        58.5,
        "turn_duration",
        json!({"durationMs": 30_500, "messageCount": 12}),
    );
    let entry = registry_entry(&s, "idle", 59.0);
    s.write(59.0, &registry, entry);

    // --- Codex root + judge ---------------------------------------------------------
    root.session_meta(&mut s, 1.0, json!("cli"), json!({"thread_source": "user"}));
    root.turn_start(&mut s, 1.2);
    root.response(
        &mut s,
        1.5,
        json!({"type": "message", "role": "user",
               "content": [{"type": "input_text", "text": "Review the tokenizer and have a judge double-check."}],
               "internal_chat_message_metadata_passthrough": {"turn_id": "turn-d1", "content_item_kinds": ["user.text"]}}),
    );
    root.exec(
        &mut s,
        3.0,
        "call_demo_ls",
        "ls src",
        1.0,
        ("lexer.rs\nparser.rs\ntokenizer.rs\n", 0),
    );
    root.exec(
        &mut s,
        6.0,
        "call_demo_test",
        "cargo test tokenizer",
        9.0,
        ("test result: ok. 18 passed\n", 0),
    );
    root.function_call(
        &mut s,
        17.0,
        "call_demo_spawn",
        "spawn_agent",
        json!({"task_name": "judge", "message": "gAAAA_SYNTHETIC_demo_spawn", "fork_turns": "none"}),
    );
    root.item(
        &mut s,
        17.5,
        json!({"type": "SubAgentActivity", "id": "call_demo_spawn", "agent_path": "/root/judge",
               "agent_thread_id": CODEX_CHILD, "kind": "started"}),
    );
    root.function_output(
        &mut s,
        18.0,
        "call_demo_spawn",
        "{\"task_name\":\"/root/judge\"}",
    );
    root.function_call(
        &mut s,
        19.0,
        "call_demo_wait",
        "wait_agent",
        json!({"timeout_ms": 60000}),
    );
    root.usage(&mut s, 19.2, 30_000);

    judge.session_meta(
        &mut s,
        18.0,
        json!({"subagent": {"thread_spawn": {"parent_thread_id": CODEX_ROOT, "depth": 1,
               "agent_path": "/root/judge", "agent_nickname": "Judge", "agent_role": null}}}),
        json!({"thread_source": "subagent", "agent_nickname": "Judge", "agent_path": "/root/judge",
               "parent_thread_id": CODEX_ROOT, "multi_agent_version": "v2"}),
    );
    judge.turn_start(&mut s, 18.5);
    judge.response(
        &mut s,
        19.0,
        json!({"type": "agent_message", "id": "amsg_demo_1", "author": "/root", "recipient": "/root/judge",
               "content": [{"type": "input_text", "text": "Message Type: NEW_TASK\nTask name: /root/judge\nSender: /root\nPayload:\n"},
                           {"type": "encrypted_content", "encrypted_content": "gAAAA_SYNTHETIC_demo_task"}]}),
    );
    judge.exec(
        &mut s,
        21.0,
        "call_demo_j_cat",
        "cat src/tokenizer.rs",
        1.5,
        ("pub fn tokenize() {}\n", 0),
    );
    judge.exec(
        &mut s,
        25.0,
        "call_demo_j_fuzz",
        "cargo fuzz run tokenizer -- -runs=1000",
        12.0,
        ("crash: slice index out of range\n", 1),
    );
    judge.usage(&mut s, 38.0, 12_000);
    let verdict = "One crash on unterminated strings; everything else looks correct.";
    judge.final_answer(&mut s, 40.0, verdict, 18.5);

    root.response(
        &mut s,
        41.0,
        json!({"type": "agent_message", "id": "amsg_demo_2", "author": "/root/judge", "recipient": "/root",
               "content": [{"type": "input_text", "text": format!("Message Type: FINAL_ANSWER\nTask name: /root/judge\nSender: /root/judge\nPayload:\n{verdict}")}]}),
    );
    root.item(
        &mut s,
        41.5,
        json!({"type": "SubAgentActivity", "id": "subagent-completed-demo", "agent_path": "/root/judge",
               "agent_thread_id": CODEX_CHILD, "kind": "completed"}),
    );
    root.function_output(
        &mut s,
        42.0,
        "call_demo_wait",
        "{\"message\":\"Wait completed.\",\"timed_out\":false}",
    );
    root.exec(
        &mut s,
        44.0,
        "call_demo_fix",
        "cargo test tokenizer",
        8.0,
        ("test result: ok. 19 passed\n", 0),
    );
    root.usage(&mut s, 53.0, 40_000);
    root.final_answer(
        &mut s,
        55.0,
        "Tokenizer fixed (unterminated strings); the judge agrees.",
        1.2,
    );

    s.steps.sort_by_key(|step| step.at);
    s.steps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_is_ordered_and_synthetic() {
        let start = DateTime::parse_from_rfc3339("2026-09-20T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
        let steps = scenario(start, 1.0, 4242, today);
        assert!(steps.windows(2).all(|w| w[0].at <= w[1].at));
        assert!(steps.last().unwrap().at <= Duration::from_secs(70));
        for step in &steps {
            if let Op::Append(line) = &step.op {
                let v: Value = serde_json::from_str(line).expect("valid JSON line");
                assert!(v.is_object());
                assert!(!line.contains("/Users/"), "no real paths: {line}");
            }
        }
    }
}
