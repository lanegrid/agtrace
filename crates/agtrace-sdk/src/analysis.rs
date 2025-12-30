use crate::error::Result;
use agtrace_engine::{AgentSession, SessionSummary};

#[derive(Debug, Clone, Copy)]
pub enum Lens {
    Failures,
    Loops,
    Bottlenecks,
}

pub struct SessionAnalyzer {
    session: AgentSession,
    lenses: Vec<Lens>,
}

impl SessionAnalyzer {
    pub(crate) fn new(session: AgentSession) -> Self {
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

    fn analyze_failures(&self) -> Vec<String> {
        let mut insights = Vec::new();
        for (idx, turn) in self.session.turns.iter().enumerate() {
            let failed_tools: Vec<_> = turn
                .steps
                .iter()
                .flat_map(|step| &step.tools)
                .filter(|tool_exec| tool_exec.is_error)
                .collect();

            if !failed_tools.is_empty() {
                insights.push(format!(
                    "Turn {}: {} tool execution(s) failed",
                    idx + 1,
                    failed_tools.len()
                ));
            }
        }
        insights
    }

    fn analyze_loops(&self) -> Vec<String> {
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
                        insights.push(format!(
                            "Potential loop detected at turn {}: repeated tool sequence",
                            idx + 1
                        ));
                    }
                } else {
                    repeat_count = 0;
                }
            }

            prev_tool_sequence = Some(tool_sequence);
        }
        insights
    }

    fn analyze_bottlenecks(&self) -> Vec<String> {
        let mut insights = Vec::new();
        for (idx, turn) in self.session.turns.iter().enumerate() {
            for step in &turn.steps {
                for tool_exec in &step.tools {
                    if let Some(duration_ms) = tool_exec.duration_ms.filter(|&d| d > 10_000) {
                        insights.push(format!(
                            "Turn {}: Slow tool execution ({:?} took {}s)",
                            idx + 1,
                            tool_exec.call.content.kind(),
                            duration_ms / 1000
                        ));
                    }
                }
            }
        }
        insights
    }

    fn calculate_health_score(&self, _summary: &SessionSummary, insights: &[String]) -> u8 {
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
    pub insights: Vec<String>,
    pub summary: SessionSummary,
}
