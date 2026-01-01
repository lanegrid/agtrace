use crate::error::Result;
use agtrace_engine::{AgentSession, SessionSummary};

/// Diagnostic lens for session analysis.
///
/// Each lens applies a specific diagnostic perspective to identify
/// potential issues or patterns in agent behavior.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum Lens {
    /// Detects tool execution failures.
    Failures,
    /// Detects repeated tool sequences that may indicate stuck behavior.
    Loops,
    /// Detects slow tool executions that may impact performance.
    Bottlenecks,
}

/// Severity level of an insight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Severity {
    /// Informational observation.
    Info,
    /// Potential issue requiring attention.
    Warning,
    /// Critical problem requiring immediate attention.
    Critical,
}

/// Diagnostic insight about a specific turn in a session.
///
/// Represents a finding from applying a diagnostic lens to a turn,
/// including the turn location, the lens that detected it, and
/// a human-readable message describing the issue.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Insight {
    /// Zero-based turn index where the insight was detected.
    pub turn_index: usize,
    /// The diagnostic lens that produced this insight.
    pub lens: Lens,
    /// Human-readable description of the finding.
    pub message: String,
    /// Severity level of this insight.
    pub severity: Severity,
}

/// Builder for analyzing sessions through multiple diagnostic lenses.
///
/// Apply one or more lenses to a session to generate an analysis report
/// with insights and a health score.
pub struct SessionAnalyzer {
    session: AgentSession,
    lenses: Vec<Lens>,
}

impl SessionAnalyzer {
    pub fn new(session: AgentSession) -> Self {
        Self {
            session,
            lenses: vec![],
        }
    }

    pub fn through(mut self, lens: Lens) -> Self {
        self.lenses.push(lens);
        self
    }

    pub fn report(self) -> Result<AnalysisReport> {
        let summary = agtrace_engine::session::summarize(&self.session);
        let mut insights = Vec::new();

        for lens in &self.lenses {
            match lens {
                Lens::Failures => {
                    insights.extend(self.analyze_failures());
                }
                Lens::Loops => {
                    insights.extend(self.analyze_loops());
                }
                Lens::Bottlenecks => {
                    insights.extend(self.analyze_bottlenecks());
                }
            }
        }

        let score = self.calculate_health_score(&summary, &insights);

        Ok(AnalysisReport {
            score,
            insights,
            summary,
        })
    }

    fn analyze_failures(&self) -> Vec<Insight> {
        let mut insights = Vec::new();
        for (idx, turn) in self.session.turns.iter().enumerate() {
            let failed_tools: Vec<_> = turn
                .steps
                .iter()
                .flat_map(|step| &step.tools)
                .filter(|tool_exec| tool_exec.is_error)
                .collect();

            if !failed_tools.is_empty() {
                insights.push(Insight {
                    turn_index: idx,
                    lens: Lens::Failures,
                    message: format!("{} tool execution(s) failed", failed_tools.len()),
                    severity: Severity::Critical,
                });
            }
        }
        insights
    }

    fn analyze_loops(&self) -> Vec<Insight> {
        let mut insights = Vec::new();
        let mut prev_tool_sequence: Option<Vec<String>> = None;
        let mut repeat_count = 0;

        for (idx, turn) in self.session.turns.iter().enumerate() {
            let tool_sequence: Vec<String> = turn
                .steps
                .iter()
                .flat_map(|step| &step.tools)
                .map(|tool_exec| format!("{:?}", tool_exec.call.content.kind()))
                .collect();

            if let Some(prev) = &prev_tool_sequence {
                if prev == &tool_sequence && !tool_sequence.is_empty() {
                    repeat_count += 1;
                    if repeat_count >= 2 {
                        insights.push(Insight {
                            turn_index: idx,
                            lens: Lens::Loops,
                            message: "Repeated tool sequence detected".to_string(),
                            severity: Severity::Warning,
                        });
                    }
                } else {
                    repeat_count = 0;
                }
            }

            prev_tool_sequence = Some(tool_sequence);
        }
        insights
    }

    fn analyze_bottlenecks(&self) -> Vec<Insight> {
        let mut insights = Vec::new();
        for (idx, turn) in self.session.turns.iter().enumerate() {
            for step in &turn.steps {
                for tool_exec in &step.tools {
                    if let Some(duration_ms) = tool_exec.duration_ms.filter(|&d| d > 10_000) {
                        insights.push(Insight {
                            turn_index: idx,
                            lens: Lens::Bottlenecks,
                            message: format!(
                                "Slow tool execution ({:?} took {}s)",
                                tool_exec.call.content.kind(),
                                duration_ms / 1000
                            ),
                            severity: Severity::Warning,
                        });
                    }
                }
            }
        }
        insights
    }

    fn calculate_health_score(&self, _summary: &SessionSummary, insights: &[Insight]) -> u8 {
        let base_score = 100;
        let penalty_per_insight = 10;
        let total_penalty = (insights.len() as i32) * penalty_per_insight;
        let score = base_score - total_penalty;
        score.max(0) as u8
    }
}

/// Comprehensive analysis report for a session.
///
/// Contains a health score (0-100), detailed insights from applied lenses,
/// and a statistical summary of the session.
#[derive(Debug, serde::Serialize)]
pub struct AnalysisReport {
    /// Health score (0-100) where 100 indicates no issues detected.
    pub score: u8,
    /// Diagnostic insights collected from all applied lenses.
    pub insights: Vec<Insight>,
    /// Statistical summary of the session.
    pub summary: SessionSummary,
}
