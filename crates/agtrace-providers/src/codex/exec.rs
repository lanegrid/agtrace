//! Codex `exec` tool: command extraction from the JS program and correlation of
//! `item_completed` sub-actions (CommandExecution, FileChange, ...) to the enclosing call.
//!
//! Sub-items do not carry the `custom_tool_call.call_id`; they are written between the
//! call and its output and share its `turn_id`. A long-running script answers with
//! `Script running with cell ID N` and keeps producing sub-items until a later
//! `wait{cell_id: N}` call reports completion.
//!
//! Items of a finished script are frequently written right *after* its output (observed in
//! Codex 0.153–0.155: `custom_tool_call_output` → `token_count` → `CommandExecution`), and
//! background sessions polled via `write_stdin` report their commands on completion. When no
//! exec is open in the item's turn, the item is therefore attributed to the most recently
//! closed exec of that turn.

use regex::Regex;
use std::sync::LazyLock;
use uuid::Uuid;

/// First `tools.exec_command({... "cmd": "<literal>" ...})` in an exec script.
static EXEC_COMMAND_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"tools\.exec_command\(\s*\{[^{}]*?"?cmd"?\s*:\s*("(?:[^"\\]|\\.)*")"#).unwrap()
});

static CELL_ID_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Script running with cell ID (\d+)").unwrap());

/// Upper bound of simultaneously tracked open exec calls (calls without output are dropped
/// oldest-first beyond this).
const MAX_OPEN: usize = 64;

/// Extract the first `tools.exec_command` `cmd` literal from an exec JS program.
pub(crate) fn extract_exec_command(script: &str) -> Option<String> {
    let caps = EXEC_COMMAND_REGEX.captures(script)?;
    serde_json::from_str::<String>(caps.get(1)?.as_str()).ok()
}

/// `Some(N)` when an exec (or wait) output says the script is still running in cell N.
pub(crate) fn running_cell_id(output: &str) -> Option<u64> {
    CELL_ID_REGEX
        .captures(output.trim_start())
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

/// True when an exec output reports a failed or aborted script.
pub(crate) fn is_failed_exec_output(output: &str) -> bool {
    let s = output.trim_start();
    s.starts_with("Script failed") || s.starts_with("aborted by user")
}

#[derive(Debug, Clone)]
struct OpenExec {
    turn_id: Option<String>,
    call_id: String,
    event_id: Uuid,
    cell: Option<u64>,
}

/// Tracks exec calls that may still receive sub-items.
#[derive(Debug, Default)]
pub(crate) struct ExecTracker {
    open: Vec<OpenExec>,
    /// `wait` call_id -> cell id it polls.
    waits: Vec<(String, u64)>,
    /// Most recently closed exec (trailing items attach to it).
    last_closed: Option<OpenExec>,
}

impl ExecTracker {
    /// An exec call was seen: it is open until its output.
    pub fn on_call(&mut self, turn_id: Option<String>, call_id: &str, event_id: Uuid) {
        if self.open.len() >= MAX_OPEN {
            self.open.remove(0);
        }
        self.open.push(OpenExec {
            turn_id,
            call_id: call_id.to_string(),
            event_id,
            cell: None,
        });
    }

    /// The exec call's output: closes it unless the script keeps running in a cell.
    pub fn on_output(&mut self, call_id: &str, output: &str) {
        let Some(idx) = self.open.iter().position(|e| e.call_id == call_id) else {
            return;
        };
        match running_cell_id(output) {
            Some(cell) => self.open[idx].cell = Some(cell),
            None => {
                self.last_closed = Some(self.open.remove(idx));
            }
        }
    }

    /// A `wait{cell_id}` call was seen.
    pub fn on_wait_call(&mut self, call_id: &str, cell_id: u64) {
        if self.waits.len() >= MAX_OPEN {
            self.waits.remove(0);
        }
        self.waits.push((call_id.to_string(), cell_id));
    }

    /// Output of a `wait` call: a non-running answer completes the polled cell.
    /// Returns true when `call_id` was a tracked wait.
    pub fn on_wait_output(&mut self, call_id: &str, output: &str) -> bool {
        let Some(idx) = self.waits.iter().position(|(c, _)| c == call_id) else {
            return false;
        };
        let (_, cell) = self.waits.remove(idx);
        if running_cell_id(output).is_none()
            && let Some(idx) = self.open.iter().position(|e| e.cell == Some(cell))
        {
            self.last_closed = Some(self.open.remove(idx));
        }
        true
    }

    /// The exec call a sub-item of `turn_id` belongs to: the most recent open exec in the
    /// same turn (any open exec when the item has no turn id), else the most recently closed
    /// exec of that turn (trailing items).
    pub fn attach(&self, turn_id: Option<&str>) -> Option<Uuid> {
        let same_turn = |e: &&OpenExec| turn_id.is_none() || e.turn_id.as_deref() == turn_id;
        self.open
            .iter()
            .rev()
            .find(same_turn)
            .or_else(|| self.last_closed.as_ref().filter(same_turn))
            .map(|e| e.event_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_first_exec_command_literal() {
        let js = r#"const r = await tools.exec_command({"cmd":"sed -n '1,20p' \"a b.md\"","workdir":"/work/demo-project","yield_time_ms":10000}); text(r.output);
const s = await tools.exec_command({"cmd":"ls"});"#;
        assert_eq!(
            extract_exec_command(js).as_deref(),
            Some(r#"sed -n '1,20p' "a b.md""#)
        );
    }

    #[test]
    fn exec_command_with_cmd_not_first_key() {
        let js = r#"await tools.exec_command({ workdir: "/work", cmd: "cargo test" })"#;
        assert_eq!(extract_exec_command(js).as_deref(), Some("cargo test"));
    }

    #[test]
    fn no_exec_command() {
        assert_eq!(
            extract_exec_command(r#"await tools.apply_patch("*** Begin Patch")"#),
            None
        );
    }

    #[test]
    fn running_cell() {
        assert_eq!(
            running_cell_id("Script running with cell ID 11\nWall time 10.0 seconds"),
            Some(11)
        );
        assert_eq!(running_cell_id("Script completed\nWall time 0.1"), None);
    }

    #[test]
    fn sub_items_attach_to_open_exec_in_same_turn() {
        let mut t = ExecTracker::default();
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        t.on_call(Some("turn-1".into()), "call_a", a);
        assert_eq!(t.attach(Some("turn-1")), Some(a));
        assert_eq!(t.attach(Some("turn-2")), None);
        t.on_output(
            "call_a",
            "Script completed\nWall time 0.1 seconds\nOutput:\n",
        );
        assert_eq!(
            t.attach(Some("turn-1")),
            Some(a),
            "trailing items after the output still belong to the last exec of the turn"
        );

        t.on_call(Some("turn-2".into()), "call_b", b);
        assert_eq!(t.attach(None), Some(b));
        assert_eq!(t.attach(Some("turn-2")), Some(b));
        t.on_output("call_b", "Script completed");
        assert_eq!(
            t.attach(Some("turn-1")),
            None,
            "other turn's exec is not reused"
        );
    }

    #[test]
    fn running_cell_stays_open_until_wait_completes() {
        let mut t = ExecTracker::default();
        let a = Uuid::from_u128(1);
        t.on_call(Some("turn-1".into()), "call_a", a);
        t.on_output(
            "call_a",
            "Script running with cell ID 7\nWall time 10.0 seconds",
        );
        assert_eq!(t.attach(Some("turn-1")), Some(a), "still running");

        t.on_wait_call("call_w1", 7);
        assert!(t.on_wait_output("call_w1", "Script running with cell ID 7\nWall time 20.0"));
        assert_eq!(
            t.attach(Some("turn-1")),
            Some(a),
            "wait reported still running"
        );

        t.on_wait_call("call_w2", 7);
        assert!(t.on_wait_output("call_w2", "Script completed\nWall time 25.0 seconds"));
        t.on_call(Some("turn-1".into()), "call_b", Uuid::from_u128(2));
        assert_eq!(
            t.attach(Some("turn-1")),
            Some(Uuid::from_u128(2)),
            "newest open exec wins"
        );
        assert!(!t.on_wait_output("call_other", "x"));
    }
}
