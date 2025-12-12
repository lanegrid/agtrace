use agtrace_types::{AgentEventV1, EventType};

#[derive(Debug, Clone, Copy)]
pub enum ExportStrategy {
    Raw,
    Clean,
    Reasoning,
}

impl ExportStrategy {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "raw" => Some(ExportStrategy::Raw),
            "clean" => Some(ExportStrategy::Clean),
            "reasoning" => Some(ExportStrategy::Reasoning),
            _ => None,
        }
    }
}

pub fn transform(events: &[AgentEventV1], strategy: ExportStrategy) -> Vec<AgentEventV1> {
    match strategy {
        ExportStrategy::Raw => events.to_vec(),
        ExportStrategy::Clean => apply_clean_strategy(events),
        ExportStrategy::Reasoning => apply_reasoning_strategy(events),
    }
}

fn apply_clean_strategy(events: &[AgentEventV1]) -> Vec<AgentEventV1> {
    let mut cleaned = Vec::new();
    let mut skip_until_next_success = false;

    for event in events.iter() {
        match event.event_type {
            EventType::ToolResult => {
                if let Some(exit_code) = event.tool_exit_code {
                    if exit_code != 0 {
                        skip_until_next_success = true;
                        continue;
                    } else {
                        skip_until_next_success = false;
                    }
                }
            }
            EventType::AssistantMessage => {
                if let Some(text) = &event.text {
                    let text_lower = text.to_lowercase();
                    if text_lower.contains("i apologize")
                        || text_lower.contains("my mistake")
                        || text_lower.contains("sorry")
                    {
                        continue;
                    }
                }
            }
            _ => {}
        }

        if !skip_until_next_success {
            let mut cleaned_event = event.clone();

            if let Some(text) = &cleaned_event.text {
                if text.len() > 5000 {
                    cleaned_event.text = Some(format!(
                        "{}...<truncated_output_for_training>",
                        text.chars().take(1000).collect::<String>()
                    ));
                }
            }

            cleaned.push(cleaned_event);
        }
    }

    cleaned
}

fn apply_reasoning_strategy(events: &[AgentEventV1]) -> Vec<AgentEventV1> {
    let mut reasoning_pairs = Vec::new();

    for i in 0..events.len() {
        if matches!(events[i].event_type, EventType::Reasoning) {
            reasoning_pairs.push(events[i].clone());

            if let Some(next) = events.get(i + 1) {
                if matches!(next.event_type, EventType::ToolCall) {
                    reasoning_pairs.push(next.clone());
                }
            }
        }
    }

    reasoning_pairs
}
