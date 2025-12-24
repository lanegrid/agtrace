use super::metrics::{SessionMetrics, compute_metrics};
use crate::AgentSession;

#[derive(Debug, Clone)]
pub struct SessionDigest {
    pub session_id: String,
    pub source: String,
    pub session: AgentSession,
    pub opening: Option<String>,
    pub activation: Option<String>,
    pub metrics: SessionMetrics,
    pub recency_boost: u32,
    pub selection_reason: Option<String>,
}

impl SessionDigest {
    pub fn new(
        session_id: &str,
        provider: &str,
        session: AgentSession,
        recency_boost: u32,
    ) -> Self {
        let opening = session
            .turns
            .first()
            .map(|t| clean_snippet(&t.user.content.text))
            .filter(|s| !s.is_empty())
            .map(|s| truncate_string(&s, 100));

        let metrics = compute_metrics(&session);
        let activation = find_activation(&session);

        Self {
            session_id: session_id.to_string(),
            source: provider.to_string(),
            session,
            opening,
            activation,
            metrics,
            recency_boost,
            selection_reason: None,
        }
    }
}

fn clean_snippet(text: &str) -> String {
    let mut cleaned = text.to_string();

    let noise_tags = [
        ("<environment_context>", "</environment_context>"),
        ("<command_output>", "</command_output>"),
        ("<changed_files>", "</changed_files>"),
    ];

    for (start_tag, end_tag) in noise_tags {
        while let Some(start_idx) = cleaned.find(start_tag) {
            if let Some(end_idx) = cleaned[start_idx..].find(end_tag) {
                let absolute_end = start_idx + end_idx + end_tag.len();
                cleaned.replace_range(start_idx..absolute_end, " [..meta..] ");
            } else {
                break;
            }
        }
    }

    let fields: Vec<&str> = cleaned.split_whitespace().collect();
    fields.join(" ")
}

fn find_activation(session: &AgentSession) -> Option<String> {
    if session.turns.is_empty() {
        return None;
    }

    let (best_idx, max_tools) = session
        .turns
        .iter()
        .enumerate()
        .map(|(i, turn)| {
            let tool_count: usize = turn.steps.iter().map(|s| s.tools.len()).sum();
            (i, tool_count)
        })
        .max_by_key(|(_, count)| *count)
        .unwrap_or((0, 0));

    if max_tools < 3 {
        return None;
    }

    session
        .turns
        .get(best_idx)
        .map(|turn| clean_snippet(&turn.user.content.text))
        .map(|s| truncate_string(&s, 120))
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let chars: Vec<char> = s.chars().take(max_len).collect();
        chars.iter().collect::<String>() + "..."
    }
}
