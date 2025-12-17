use agtrace_types::{AgentEvent, EventPayload};
use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub enum ExportStrategy {
    Raw,
    Clean,
    Reasoning,
}

impl FromStr for ExportStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "raw" => Ok(ExportStrategy::Raw),
            "clean" => Ok(ExportStrategy::Clean),
            "reasoning" => Ok(ExportStrategy::Reasoning),
            _ => Err(format!("Unknown export strategy: {}", s)),
        }
    }
}

pub fn transform(events: &[AgentEvent], strategy: ExportStrategy) -> Vec<AgentEvent> {
    match strategy {
        ExportStrategy::Raw => events.to_vec(),
        ExportStrategy::Clean => apply_clean_strategy(events),
        ExportStrategy::Reasoning => apply_reasoning_strategy(events),
    }
}

fn apply_clean_strategy(events: &[AgentEvent]) -> Vec<AgentEvent> {
    let mut cleaned = Vec::new();
    let mut skip_until_next_success = false;

    for event in events.iter() {
        match &event.payload {
            EventPayload::ToolResult(result) => {
                if result.is_error {
                    skip_until_next_success = true;
                    continue;
                } else {
                    skip_until_next_success = false;
                }
            }
            EventPayload::Message(msg) => {
                let text_lower = msg.text.to_lowercase();
                if text_lower.contains("i apologize")
                    || text_lower.contains("my mistake")
                    || text_lower.contains("sorry")
                {
                    continue;
                }
            }
            _ => {}
        }

        if !skip_until_next_success {
            let mut cleaned_event = event.clone();

            // Truncate long text in payloads
            match &mut cleaned_event.payload {
                EventPayload::Message(msg) => {
                    if msg.text.len() > 5000 {
                        msg.text = format!(
                            "{}...<truncated_output_for_training>",
                            msg.text.chars().take(1000).collect::<String>()
                        );
                    }
                }
                EventPayload::ToolResult(result) => {
                    if result.output.len() > 5000 {
                        result.output = format!(
                            "{}...<truncated_output_for_training>",
                            result.output.chars().take(1000).collect::<String>()
                        );
                    }
                }
                EventPayload::Reasoning(reasoning) => {
                    if reasoning.text.len() > 5000 {
                        reasoning.text = format!(
                            "{}...<truncated_output_for_training>",
                            reasoning.text.chars().take(1000).collect::<String>()
                        );
                    }
                }
                _ => {}
            }

            cleaned.push(cleaned_event);
        }
    }

    cleaned
}

fn apply_reasoning_strategy(events: &[AgentEvent]) -> Vec<AgentEvent> {
    let mut reasoning_pairs = Vec::new();

    for i in 0..events.len() {
        if matches!(events[i].payload, EventPayload::Reasoning(_)) {
            reasoning_pairs.push(events[i].clone());

            if let Some(next) = events.get(i + 1) {
                if matches!(next.payload, EventPayload::ToolCall(_)) {
                    reasoning_pairs.push(next.clone());
                }
            }
        }
    }

    reasoning_pairs
}
