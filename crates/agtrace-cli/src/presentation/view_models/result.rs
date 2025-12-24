use serde::Serialize;

use super::common::{Guidance, StatusBadge};

#[derive(Debug, Serialize)]
pub struct CommandResultViewModel<T>
where
    T: Serialize,
{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge: Option<StatusBadge>,

    pub content: T,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<Guidance>,
}

impl<T> CommandResultViewModel<T>
where
    T: Serialize,
{
    pub fn new(content: T) -> Self {
        Self {
            badge: None,
            content,
            suggestions: Vec::new(),
        }
    }

    pub fn with_badge(mut self, badge: StatusBadge) -> Self {
        self.badge = Some(badge);
        self
    }

    pub fn with_suggestion(mut self, guide: Guidance) -> Self {
        self.suggestions.push(guide);
        self
    }

    pub fn with_suggestions(mut self, guides: Vec<Guidance>) -> Self {
        self.suggestions.extend(guides);
        self
    }
}
