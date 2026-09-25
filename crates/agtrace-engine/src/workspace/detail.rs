//! Per-agent history and detail state behind the watch overview and agent detail
//! screens: activity buckets, status history, context series, instructions,
//! result, last assistant text and usage totals.
//!
//! Everything here is folded incrementally (O(1) amortised per event) and bounded:
//! activity is kept per minute for [`ACTIVITY_RETENTION`], the other series are
//! rings, and texts are capped at [`DETAIL_TEXT_MAX`] bytes.

use std::collections::VecDeque;

use agtrace_types::{AgentMessageKind, TokenUsagePayload};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::ring::RingBuffer;
use super::status::AgentStatus;

/// Width of one activity bucket.
pub const ACTIVITY_BUCKET: Duration = Duration::minutes(1);
/// Activity older than this (relative to the agent's newest bucket) is dropped.
pub const ACTIVITY_RETENTION: Duration = Duration::hours(24);
/// Status transitions kept per agent.
pub const STATUS_HISTORY_CAPACITY: usize = 256;
/// Context samples (usage / compaction) kept per agent.
pub const CONTEXT_SERIES_CAPACITY: usize = 256;
/// Instructions kept per agent after the initial one (which is never evicted).
pub const INSTRUCTION_CAPACITY: usize = 32;
/// Maximum bytes kept of an instruction, result or assistant text.
pub const DETAIL_TEXT_MAX: usize = 16 * 1024;
/// Usage dedupe keys remembered for totals (re-emitted records are near each other).
const RECENT_USAGE_KEYS: usize = 64;

/// `s` cut to at most `max` bytes on a char boundary (with a trailing `…` when cut).
pub fn cap_text(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max.saturating_sub('…'.len_utf8());
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = s[..end].to_string();
    out.push('…');
    out
}

// ---------------------------------------------------------------- activity

/// Own-log events of one minute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityBucket {
    /// Start of the minute.
    pub start: DateTime<Utc>,
    /// Activity events (tool calls, messages, prompts, ...; not usage / metadata).
    pub events: u32,
    pub compactions: u32,
}

/// Sparse per-minute activity counts (only minutes with events are stored).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActivityHistory {
    buckets: VecDeque<ActivityBucket>,
}

fn floor_bucket(ts: DateTime<Utc>) -> DateTime<Utc> {
    let secs = ACTIVITY_BUCKET.num_seconds();
    DateTime::from_timestamp(ts.timestamp().div_euclid(secs) * secs, 0).unwrap_or(ts)
}

impl ActivityHistory {
    /// Count one event at `ts` (a compaction also counts as a compaction).
    pub fn record(&mut self, ts: DateTime<Utc>, compaction: bool) {
        let start = floor_bucket(ts);
        if self
            .buckets
            .back()
            .is_some_and(|b| start < b.start - ACTIVITY_RETENTION)
        {
            return;
        }
        // Events arrive in file order: the bucket is almost always the last one.
        let pos = self.buckets.iter().rposition(|b| b.start <= start);
        let bucket = match pos {
            Some(i) if self.buckets[i].start == start => &mut self.buckets[i],
            Some(i) => {
                self.buckets.insert(i + 1, empty_bucket(start));
                &mut self.buckets[i + 1]
            }
            None => {
                self.buckets.push_front(empty_bucket(start));
                &mut self.buckets[0]
            }
        };
        if compaction {
            bucket.compactions += 1;
        } else {
            bucket.events += 1;
        }
        let newest = self.buckets.back().map(|b| b.start).unwrap_or(start);
        while self
            .buckets
            .front()
            .is_some_and(|b| b.start < newest - ACTIVITY_RETENTION)
        {
            self.buckets.pop_front();
        }
    }

    /// Buckets whose minute starts in `[from, to)`, oldest first.
    pub fn between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> impl Iterator<Item = &ActivityBucket> {
        let lo = self
            .buckets
            .partition_point(|b| b.start < floor_bucket(from));
        self.buckets.range(lo..).take_while(move |b| b.start < to)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ActivityBucket> {
        self.buckets.iter()
    }

    pub fn len(&self) -> usize {
        self.buckets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buckets.is_empty()
    }

    pub fn clear(&mut self) {
        self.buckets.clear();
    }
}

fn empty_bucket(start: DateTime<Utc>) -> ActivityBucket {
    ActivityBucket {
        start,
        events: 0,
        compactions: 0,
    }
}

// ---------------------------------------------------------------- status history

/// The agent entered `status` at `at` (event time).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusPoint {
    pub at: DateTime<Utc>,
    pub status: AgentStatus,
}

/// Status transitions in event time: own-log signals (activity ⇒ running, turn end
/// ⇒ idle, own lifecycle) and parent-side signals (idle / done / killed reports).
///
/// This is the history of the *signals*; the current effective status (which may
/// also come from the process registry or staleness) is [`super::AgentView::status`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StatusHistory {
    points: RingBuffer<StatusPoint, STATUS_HISTORY_CAPACITY>,
}

impl StatusHistory {
    /// Own-log signal: the agent was in `status` from `at`. Skipped when the point
    /// before it (in time) already has that status, so the frequent "activity ⇒
    /// running" signals add a point per transition only.
    pub fn record_own(&mut self, at: DateTime<Utc>, status: AgentStatus) {
        let point = StatusPoint { at, status };
        match self.points.last() {
            None => self.points.push(point),
            Some(last) if last.at <= at => {
                if last.status != status {
                    self.points.push(point);
                }
            }
            Some(_) => {
                if self.at(at) != Some(status) {
                    self.points.insert_by_key(point, |p| p.at);
                }
            }
        }
    }

    /// Report from another agent's log (done, killed, idle, ...). Always kept (the
    /// own log may be folded later and put earlier points between two reports);
    /// only an identical point is skipped (replays).
    pub fn record_report(&mut self, at: DateTime<Utc>, status: AgentStatus) {
        let point = StatusPoint { at, status };
        if self.points.iter().rev().any(|p| *p == point) {
            return;
        }
        self.points.insert_by_key(point, |p| p.at);
    }

    /// Status in force at `t` (None before the first point).
    pub fn at(&self, t: DateTime<Utc>) -> Option<AgentStatus> {
        self.points
            .iter()
            .rev()
            .find(|p| p.at <= t)
            .map(|p| p.status)
    }

    /// Start of the latest run of consecutive points with `status`.
    pub fn entered(&self, status: AgentStatus) -> Option<DateTime<Utc>> {
        let mut out = None;
        for p in self.points.iter().rev() {
            if p.status != status {
                break;
            }
            out = Some(p.at);
        }
        out
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &StatusPoint> {
        self.points.iter()
    }

    pub fn last(&self) -> Option<&StatusPoint> {
        self.points.last()
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }
}

// ---------------------------------------------------------------- context series

/// One context-occupancy sample: a usage record, or a compaction boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextPoint {
    pub at: DateTime<Utc>,
    /// Context tokens (usage), or the post-compaction size when known.
    pub tokens: Option<u64>,
    pub compaction: bool,
}

// ---------------------------------------------------------------- instructions

/// Where an instruction came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionKind {
    /// A user prompt in the agent's own log (the task of a subagent / fork, or a
    /// human prompt of a main session).
    Prompt,
    /// A prompt queued while the agent was busy and absorbed into the running turn.
    Queued,
    /// An inter-agent message addressed to the agent (task, follow-up, peer message).
    Message(AgentMessageKind),
}

/// Something the agent was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub event_id: Uuid,
    pub at: DateTime<Utc>,
    pub kind: InstructionKind,
    /// Sender label (messages only).
    pub from: Option<String>,
    /// Text, capped at [`DETAIL_TEXT_MAX`]; None when encrypted or absent.
    pub text: Option<String>,
    pub encrypted: bool,
}

/// What the agent produced: its final answer / hand-back / task-notification result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResult {
    pub event_id: Uuid,
    pub at: DateTime<Utc>,
    pub kind: AgentMessageKind,
    pub text: Option<String>,
    pub encrypted: bool,
    /// Taken from the agent's own log (reset with it).
    pub own: bool,
}

impl AgentResult {
    fn plain(&self) -> bool {
        self.text.is_some() && !self.encrypted
    }

    /// How directly the kind carries the result: the agent's own final answer /
    /// hand-back beats a task notification (Claude's `<result>` may only say that
    /// the report was delivered elsewhere).
    fn rank(&self) -> u8 {
        match self.kind {
            AgentMessageKind::FinalAnswer | AgentMessageKind::Handback => 2,
            AgentMessageKind::TaskNotification => 1,
            _ => 0,
        }
    }

    /// `self` should replace `old`: a plaintext body beats none, then the more
    /// direct kind, then the latest.
    pub(crate) fn supersedes(&self, old: &AgentResult) -> bool {
        (self.plain(), self.rank(), self.at) >= (old.plain(), old.rank(), old.at)
    }
}

/// Latest assistant text or reasoning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    pub at: DateTime<Utc>,
    pub text: String,
}

// ---------------------------------------------------------------- totals

/// Token and call totals of the agent's own log.
///
/// Usage records re-emitted for the same request (`dedupe_key`, e.g. a Claude
/// stream-start snapshot followed by the final usage) replace their earlier value
/// instead of adding to it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageTotals {
    /// Sum of input tokens (uncached + cache read + cache write) over requests.
    pub input: u64,
    /// Sum of output tokens over requests.
    pub output: u64,
    pub turns: u32,
    pub tool_calls: u32,
    /// Requests whose latest usage is a stream-start snapshot: their output is a
    /// lower bound (the provider never logged the final count).
    pub partial_outputs: u32,
    recent: VecDeque<(String, u64, u64, bool)>,
}

impl UsageTotals {
    pub(crate) fn add_usage(&mut self, u: &TokenUsagePayload) {
        let (i, o) = (u.input.total(), u.output.total());
        let partial = !u.completeness.is_final();
        if let Some(key) = &u.dedupe_key {
            if let Some(e) = self.recent.iter_mut().find(|e| e.0 == *key) {
                self.input = self.input.saturating_sub(e.1);
                self.output = self.output.saturating_sub(e.2);
                if e.3 {
                    self.partial_outputs = self.partial_outputs.saturating_sub(1);
                }
                e.1 = i;
                e.2 = o;
                e.3 = partial;
            } else {
                if self.recent.len() == RECENT_USAGE_KEYS {
                    self.recent.pop_front();
                }
                self.recent.push_back((key.clone(), i, o, partial));
            }
        }
        if partial {
            self.partial_outputs += 1;
        }
        self.input += i;
        self.output += o;
    }
}

// ---------------------------------------------------------------- per-agent bundle

/// Detail state of one agent (see the module docs).
#[derive(Debug, Clone, Default)]
pub struct AgentDetail {
    pub activity: ActivityHistory,
    pub status_history: StatusHistory,
    pub context_series: RingBuffer<ContextPoint, CONTEXT_SERIES_CAPACITY>,
    /// Number of compactions seen in the own log.
    pub compactions: u32,
    /// The first instruction (kept even when later ones are evicted).
    pub initial_task: Option<Instruction>,
    /// Later instructions, oldest first.
    pub instructions: RingBuffer<Instruction, INSTRUCTION_CAPACITY>,
    pub result: Option<AgentResult>,
    pub last_message: Option<Said>,
    pub last_reasoning: Option<Said>,
    /// Why the agent ended (failed turn error, kill / failure reason).
    pub end_reason: Option<String>,
    pub totals: UsageTotals,
}

impl AgentDetail {
    /// Initial task followed by the later instructions.
    pub fn all_instructions(&self) -> impl Iterator<Item = &Instruction> {
        self.initial_task.iter().chain(self.instructions.iter())
    }

    pub(crate) fn push_instruction(&mut self, i: Instruction) {
        if self.all_instructions().any(|x| x.event_id == i.event_id) {
            return;
        }
        if self.initial_task.is_none() {
            self.initial_task = Some(i);
        } else {
            self.instructions.push(i);
        }
    }

    pub(crate) fn set_result(&mut self, r: AgentResult) {
        if self.result.as_ref().is_none_or(|old| r.supersedes(old)) {
            self.result = Some(r);
        }
    }

    pub(crate) fn push_context(&mut self, p: ContextPoint) {
        if self.context_series.last() == Some(&p) {
            return;
        }
        self.context_series.push(p);
    }

    /// Forget everything derived from the agent's own log (file truncated /
    /// replaced). Results and status reported by other agents are kept.
    pub(crate) fn reset_own(&mut self) {
        let result = self.result.take().filter(|r| !r.own);
        let end_reason = self.end_reason.take();
        *self = AgentDetail {
            result,
            end_reason,
            ..AgentDetail::default()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(secs: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 20, 12, 0, 0).unwrap() + Duration::seconds(secs)
    }

    #[test]
    fn activity_buckets_per_minute_and_out_of_order() {
        let mut a = ActivityHistory::default();
        a.record(t(1), false);
        a.record(t(59), false);
        a.record(t(61), false);
        a.record(t(130), true);
        // Late event of an earlier minute goes into its own bucket.
        a.record(t(30), false);
        let got: Vec<(i64, u32, u32)> = a
            .iter()
            .map(|b| ((b.start - t(0)).num_seconds(), b.events, b.compactions))
            .collect();
        assert_eq!(got, vec![(0, 3, 0), (60, 1, 0), (120, 0, 1)]);
        let mid: Vec<i64> = a
            .between(t(60), t(120))
            .map(|b| (b.start - t(0)).num_seconds())
            .collect();
        assert_eq!(mid, vec![60]);
    }

    #[test]
    fn activity_is_bounded_by_retention() {
        let mut a = ActivityHistory::default();
        a.record(t(0), false);
        a.record(t(60), false);
        let later = ACTIVITY_RETENTION.num_seconds() + 90;
        a.record(t(later), false);
        assert_eq!(a.len(), 2, "the first minute fell out of the window");
        // Events older than the retention window are ignored.
        a.record(t(0), false);
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn status_history_dedupes_own_signals_and_answers_point_queries() {
        let mut h = StatusHistory::default();
        h.record_own(t(0), AgentStatus::Running);
        h.record_own(t(5), AgentStatus::Running);
        h.record_own(t(10), AgentStatus::Idle);
        h.record_own(t(20), AgentStatus::Running);
        assert_eq!(h.iter().count(), 3);
        assert_eq!(h.at(t(-1)), None);
        assert_eq!(h.at(t(12)), Some(AgentStatus::Idle));
        assert_eq!(h.entered(AgentStatus::Running), Some(t(20)));
        // A parent-side report older than the newest point is inserted in order.
        h.record_report(t(15), AgentStatus::Done);
        assert_eq!(h.at(t(16)), Some(AgentStatus::Done));
        assert_eq!(h.at(t(21)), Some(AgentStatus::Running));
    }

    /// Regression (real data): the parent reported a subagent done twice before the
    /// subagent's own log was folded. The second report was dropped as a repeat,
    /// so the own log's activity in between left the agent "running" forever.
    #[test]
    fn reports_survive_own_log_folded_later() {
        let mut h = StatusHistory::default();
        h.record_own(t(0), AgentStatus::Running);
        h.record_report(t(50), AgentStatus::Done);
        h.record_report(t(100), AgentStatus::Done);
        h.record_report(t(100), AgentStatus::Done); // replay
        // The subagent was resumed between the two reports.
        h.record_own(t(80), AgentStatus::Running);
        h.record_own(t(90), AgentStatus::Running);
        assert_eq!(h.at(t(85)), Some(AgentStatus::Running));
        assert_eq!(h.at(t(200)), Some(AgentStatus::Done));
        assert_eq!(h.iter().count(), 4);
    }

    #[test]
    fn usage_totals_replace_re_emitted_requests() {
        use agtrace_types::{TokenInput, TokenOutput};
        let u = |i: u64, o: u64, key: Option<&str>| {
            TokenUsagePayload::new(TokenInput::new(i, 0, 0), TokenOutput::new(o, 0, 0))
                .with_dedupe_key(key.map(str::to_string))
        };
        let mut tot = UsageTotals::default();
        tot.add_usage(&u(100, 1, Some("m1")));
        tot.add_usage(&u(100, 40, Some("m1"))); // final usage of the same request
        tot.add_usage(&u(200, 5, Some("m2")));
        tot.add_usage(&u(10, 1, None));
        assert_eq!((tot.input, tot.output), (310, 46));
        assert_eq!(tot.partial_outputs, 0);
        // A stream-start snapshot is a lower bound until its final usage arrives.
        let partial = |key: &str| {
            u(50, 2, Some(key)).with_completeness(agtrace_types::UsageCompleteness::PartialOutput)
        };
        tot.add_usage(&partial("m3"));
        tot.add_usage(&partial("m4"));
        assert_eq!(tot.partial_outputs, 2);
        tot.add_usage(&u(50, 30, Some("m3")));
        assert_eq!(tot.partial_outputs, 1);
    }

    #[test]
    fn cap_text_cuts_on_char_boundary() {
        assert_eq!(cap_text("  short ", 10), "short");
        let s = "日本語テキスト";
        let c = cap_text(s, 10);
        assert!(c.len() <= 10, "{c}");
        assert!(c.ends_with('…'));
    }

    #[test]
    fn result_prefers_plaintext_then_latest() {
        let r = |at: i64, text: Option<&str>, encrypted: bool| AgentResult {
            event_id: Uuid::nil(),
            at: t(at),
            kind: AgentMessageKind::FinalAnswer,
            text: text.map(str::to_string),
            encrypted,
            own: false,
        };
        let mut d = AgentDetail::default();
        d.set_result(r(10, Some("a"), false));
        d.set_result(r(20, None, true));
        assert_eq!(d.result.as_ref().unwrap().text.as_deref(), Some("a"));
        d.set_result(r(30, Some("b"), false));
        assert_eq!(d.result.as_ref().unwrap().text.as_deref(), Some("b"));
        // Regression (real data): a later task notification whose <result> only
        // says "delivered as a message" must not replace the hand-back text.
        d.set_result(AgentResult {
            kind: AgentMessageKind::TaskNotification,
            ..r(40, Some("report was delivered to you as a message"), false)
        });
        assert_eq!(d.result.as_ref().unwrap().text.as_deref(), Some("b"));
    }
}
