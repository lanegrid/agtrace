//! Agent detail screen: who the agent is, what it was asked to do, what it is doing
//! now, what it produced, and its timeline.

use agtrace_sdk::types::{AgentKind, AgentProvider, PlanItemStatus};
use agtrace_sdk::workspace::{AgentView, Instruction, InstructionKind, PlanTask, WorkspaceView};
use chrono::{DateTime, Utc};

use super::{activity, ctx, hhmm, message_tag, status_vm, timeline_row};
use crate::presentation::view_models::watch::{
    DetailNowVm, DetailVm, InstructionVm, PlanVm, ResultVm, TaskStatusVm, TaskVm, TotalsVm, UiState,
};

/// Context sparkline width (samples).
pub const SPARK_WIDTH: usize = 24;
const SPARK_LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub(super) fn build_detail(
    view: &WorkspaceView,
    ui: &UiState,
    a: &AgentView,
    now: DateTime<Utc>,
) -> DetailVm {
    let offset = ui.utc_offset;
    let status_since = a
        .detail
        .status_history
        .entered(a.status)
        .or(a.last_active())
        .or(a.agent.started_at);
    DetailVm {
        agent_id: a.id().as_str().to_string(),
        title: a.label(),
        relation: relation(view, a),
        agent_type: a.agent.agent_type.clone(),
        model: a.model.clone(),
        effort: a.effort().map(str::to_string),
        status: status_vm(a.status),
        status_secs: status_since.map(|t| (now - t).num_seconds().max(0)),
        ctx: ctx(a),
        spark: spark(a),
        totals: TotalsVm {
            input_tokens: a.detail.totals.input,
            output_tokens: a.detail.totals.output,
            output_partial: a.detail.totals.partial_outputs > 0,
            turns: a.detail.totals.turns,
            tool_calls: a.detail.totals.tool_calls,
        },
        instructions: instructions(view, a, ui),
        now: now_section(view, a, ui, now),
        result: result(a, ui),
        timeline: a
            .recent
            .iter()
            .filter_map(|e| timeline_row(e, offset))
            .collect(),
        section: ui.detail_section,
        scroll: ui.detail_scroll,
    }
}

fn parent_label(view: &WorkspaceView, a: &AgentView) -> Option<String> {
    let p = a
        .spawn
        .as_ref()
        .map(|s| &s.by)
        .or(a.agent.parent.as_ref())
        .or(a.tree_parent.as_ref())?;
    view.agent(p).map(|v| v.label())
}

/// `teammate of lead (team t)`, `subagent of lead`, `thread /root/x of /root`, ...
fn relation(view: &WorkspaceView, a: &AgentView) -> String {
    let parent = parent_label(view, a);
    let of = |what: &str| match &parent {
        Some(p) => format!("{what} of {p}"),
        None => what.to_string(),
    };
    match (a.agent.provider, a.agent.kind) {
        (_, AgentKind::Teammate) => match a.team() {
            Some(t) => format!("{} (team {t})", of("teammate")),
            None => of("teammate"),
        },
        (_, AgentKind::Subagent) => of("subagent"),
        (_, AgentKind::Fork) => of("fork"),
        (_, AgentKind::CodexThread) => of("thread"),
        (AgentProvider::Codex, AgentKind::Main) => "codex root thread".to_string(),
        (AgentProvider::ClaudeCode, AgentKind::Main) => "main session".to_string(),
    }
}

/// Occupancy history over [`SPARK_WIDTH`] groups of samples (max per group), with
/// `⟲` for groups containing a compaction.
fn spark(a: &AgentView) -> String {
    let window = a.window.as_ref().map(|w| w.tokens).filter(|t| *t > 0);
    let points: Vec<_> = a.detail.context_series.iter().collect();
    if points.is_empty() {
        return String::new();
    }
    let groups = points.len().min(SPARK_WIDTH);
    let peak = points.iter().filter_map(|p| p.tokens).max().unwrap_or(0);
    let scale = window.unwrap_or(peak).max(1) as f64;
    (0..groups)
        .map(|g| {
            let lo = g * points.len() / groups;
            let hi = ((g + 1) * points.len() / groups).max(lo + 1);
            let group = &points[lo..hi];
            if group.iter().any(|p| p.compaction) {
                return '⟲';
            }
            let max = group.iter().filter_map(|p| p.tokens).max().unwrap_or(0);
            let level = ((max as f64 / scale) * SPARK_LEVELS.len() as f64).floor() as usize;
            SPARK_LEVELS[level.min(SPARK_LEVELS.len() - 1)]
        })
        .collect()
}

fn instruction_vm(
    a: &AgentView,
    i: &Instruction,
    first: bool,
    ui: &UiState,
    parent: Option<&str>,
) -> InstructionVm {
    let me = a.label();
    let header = match &i.kind {
        InstructionKind::Prompt if a.agent.kind == AgentKind::Main => "user".to_string(),
        InstructionKind::Prompt => match (first, parent) {
            (true, Some(p)) => format!("spawned by {p}"),
            _ => "prompt".to_string(),
        },
        InstructionKind::Queued => "queued prompt".to_string(),
        InstructionKind::Message(k) => {
            let from = i.from.as_deref().unwrap_or("?");
            if first && a.spawn.is_some() {
                format!("spawned by {from} · {}", message_tag(k))
            } else {
                format!("{from} → {me} · {}", message_tag(k))
            }
        }
    };
    let note = (i.encrypted && i.text.is_none()).then(|| encrypted_note(a, i));
    InstructionVm {
        time: hhmm(i.at, ui.utc_offset),
        header,
        text: i.text.clone(),
        encrypted: i.encrypted,
        note,
    }
}

/// What is known about an encrypted (Codex) task besides its body.
fn encrypted_note(a: &AgentView, i: &Instruction) -> String {
    let lead = if a.agent.provider == AgentProvider::Codex {
        "[encrypted by Codex]"
    } else {
        "[encrypted]"
    };
    let mut parts = vec![format!(
        "{lead} {} from {}",
        match &i.kind {
            InstructionKind::Message(k) => message_tag(k),
            _ => "task".to_string(),
        },
        i.from.as_deref().unwrap_or("?")
    )];
    if let Some(p) = &a.agent.path {
        parts.push(format!("path {p}"));
    }
    if let Some(n) = a.spawn.as_ref().and_then(|s| s.name.as_deref()) {
        parts.push(format!("task {n}"));
    }
    if let Some(m) = a.spawn.as_ref().and_then(|s| s.requested_model.as_deref()) {
        parts.push(format!("requested model {m}"));
    }
    parts.join(" · ")
}

fn instructions(view: &WorkspaceView, a: &AgentView, ui: &UiState) -> Vec<InstructionVm> {
    let parent = parent_label(view, a);
    let mut out: Vec<InstructionVm> = Vec::new();
    // The spawn tool call's prompt stands in for a task the child's own log does
    // not show (yet).
    if a.detail.initial_task.is_none()
        && let Some(s) = &a.spawn
        && (s.prompt.is_some() || s.prompt_encrypted || s.description.is_some())
    {
        let from = view.agent(&s.by).map(|v| v.label());
        let text = s.prompt.clone().or_else(|| s.description.clone());
        out.push(InstructionVm {
            time: hhmm(s.at, ui.utc_offset),
            header: format!(
                "spawned by {}{}",
                from.as_deref().unwrap_or("?"),
                if s.prompt.is_none() && !s.prompt_encrypted {
                    " · description"
                } else {
                    ""
                }
            ),
            encrypted: s.prompt_encrypted && s.prompt.is_none(),
            note: (s.prompt_encrypted && s.prompt.is_none())
                .then(|| "[encrypted by Codex] spawn prompt".to_string()),
            text: if s.prompt_encrypted {
                s.prompt.clone()
            } else {
                text
            },
        });
    }
    for (n, i) in a.detail.all_instructions().enumerate() {
        out.push(instruction_vm(a, i, n == 0, ui, parent.as_deref()));
    }
    out
}

fn now_section(
    view: &WorkspaceView,
    a: &AgentView,
    ui: &UiState,
    now: DateTime<Utc>,
) -> DetailNowVm {
    let d = &a.detail;
    let (said, reasoning) = match (&d.last_message, &d.last_reasoning) {
        (Some(m), _) => (Some(m), false),
        (None, Some(r)) => (Some(r), true),
        (None, None) => (None, false),
    };
    DetailNowVm {
        status: status_vm(a.status),
        tool: a.current_tool.as_ref().and_then(|_| activity(a, now)),
        plan: plan(view, a, ui),
        said: said.map(|s| s.text.clone()),
        said_is_reasoning: reasoning,
        said_time: said.map(|s| hhmm(s.at, ui.utc_offset)),
    }
}

/// `shared`: the same task in the lead's list, for a task this agent only updated
/// (team-shared lists), to name it.
fn task_vm(t: &PlanTask, shared: Option<&PlanTask>) -> TaskVm {
    let status = match t.status {
        PlanItemStatus::Pending => TaskStatusVm::Pending,
        PlanItemStatus::InProgress => TaskStatusVm::InProgress,
        PlanItemStatus::Completed => TaskStatusVm::Completed,
        PlanItemStatus::Deleted | PlanItemStatus::Other(_) => TaskStatusVm::Other,
    };
    let subject = t
        .subject
        .as_deref()
        .or(shared.and_then(|s| s.subject.as_deref()));
    let text = match status {
        TaskStatusVm::InProgress => t
            .active_form
            .as_deref()
            .or(shared.and_then(|s| s.active_form.as_deref()))
            .or(subject),
        _ => subject,
    }
    .map(str::to_string)
    .or_else(|| t.id.as_ref().map(|id| format!("#{id}")))
    .unwrap_or_default();
    TaskVm {
        status,
        text,
        by: t.by.clone(),
    }
}

fn plan(view: &WorkspaceView, a: &AgentView, ui: &UiState) -> PlanVm {
    let p = &a.detail.plan;
    let parent = a
        .spawn
        .as_ref()
        .map(|s| &s.by)
        .or(a.agent.parent.as_ref())
        .and_then(|id| view.agent(id));
    let shared = |t: &PlanTask| {
        let id = t.id.as_deref().filter(|_| t.subject.is_none())?;
        parent?.detail.plan.task(id)
    };
    PlanVm {
        goal: p
            .goal
            .as_ref()
            .map(|g| (g.objective.clone(), g.status.clone())),
        tasks: p.tasks.iter().map(|t| task_vm(t, shared(t))).collect(),
        text: p.text.as_ref().map(|t| t.text.clone()),
        text_time: p.text.as_ref().map(|t| hhmm(t.at, ui.utc_offset)),
    }
}

fn result(a: &AgentView, ui: &UiState) -> ResultVm {
    if let Some(r) = &a.detail.result {
        return ResultVm::Reported {
            time: hhmm(r.at, ui.utc_offset),
            tag: message_tag(&r.kind),
            text: r.text.clone().filter(|_| !r.encrypted),
            encrypted: r.encrypted,
        };
    }
    if !a.status.is_terminal() {
        return ResultVm::Pending {
            status: status_vm(a.status),
        };
    }
    match &a.detail.last_message {
        Some(m) => ResultVm::LastMessage {
            time: hhmm(m.at, ui.utc_offset),
            text: m.text.clone(),
            status: status_vm(a.status),
            reason: a.detail.end_reason.clone(),
        },
        _ => ResultVm::Ended {
            status: status_vm(a.status),
            reason: a.detail.end_reason.clone(),
        },
    }
}
