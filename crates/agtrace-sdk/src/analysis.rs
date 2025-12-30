use crate::error::Result;
use agtrace_engine::{AgentSession, SessionSummary};

#[derive(Debug, Clone, Copy)]
pub enum Lens {
    Failures,
    Loops,
    Bottlenecks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone)]
pub struct Insight {
    pub turn_index: usize,
    pub lens: Lens,
    pub message: String,
    pub severity: Severity,
}

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

#[derive(Debug)]
pub struct AnalysisReport {
    pub score: u8,
    pub insights: Vec<Insight>,
    pub summary: SessionSummary,
}
