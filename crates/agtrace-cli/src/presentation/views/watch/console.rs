//! Plain-text lines of `watch --mode console` (non-TTY / CI), built from the same
//! view models as the TUI.

use crate::presentation::view_models::watch::{
    AgentRowVm, FeedRowKind, FeedRowVm, StatusVm, TimelineRowVm,
};

pub fn status_word(s: StatusVm) -> &'static str {
    match s {
        StatusVm::Running => "running",
        StatusVm::Idle => "idle",
        StatusVm::Done => "done",
        StatusVm::Killed => "killed",
        StatusVm::Failed => "failed",
        StatusVm::Unknown => "unknown",
    }
}

fn badge(row: &AgentRowVm) -> String {
    match row.badge {
        Some(b) => format!("[{b}] "),
        None if row.provider == "codex" && row.depth == 0 => "codex ".to_string(),
        None => String::new(),
    }
}

fn ctx(row: &AgentRowVm) -> String {
    row.ctx_pct.map(|p| format!(" {p}%")).unwrap_or_default()
}

/// `+ ` line for an agent seen for the first time (indented by tree depth).
pub fn agent_added(row: &AgentRowVm) -> String {
    format!(
        "+ {}{}{}  {}{}",
        "  ".repeat(row.depth as usize),
        badge(row),
        row.label,
        status_word(row.status),
        ctx(row)
    )
}

/// `~ ` line for an agent whose status changed.
pub fn agent_status(row: &AgentRowVm) -> String {
    format!(
        "~ {}{}  {}{}",
        badge(row),
        row.label,
        status_word(row.status),
        ctx(row)
    )
}

/// One timeline row of `agent`.
pub fn timeline(agent: &str, r: &TimelineRowVm) -> String {
    let label = if r.label.is_empty() {
        String::new()
    } else {
        format!("{} ", r.label)
    };
    format!("{} [{}] {} {}{}", r.time, agent, r.icon, label, r.text)
        .trim_end()
        .to_string()
}

/// One feed row (inter-agent message, spawn, lifecycle).
pub fn feed(r: &FeedRowVm) -> String {
    let route = if r.to.is_empty() {
        r.from.clone()
    } else {
        format!("{} → {}", r.from, r.to.join(", "))
    };
    let text = match (&r.text, r.encrypted) {
        (_, true) => "[encrypted]".to_string(),
        (Some(t), false) if r.kind == FeedRowKind::Message => format!("\"{t}\""),
        (Some(t), false) => t.clone(),
        (None, false) => String::new(),
    };
    format!("{} ✉ {}  {}  {}", r.time, route, r.tag, text)
        .trim_end()
        .to_string()
}
