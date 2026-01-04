use agtrace_sdk::{Client, SessionFilter};
use serde::{Deserialize, Serialize};

use super::types::{
    EventMatch, GetTurnsArgs, GetTurnsResponse, ListTurnsArgs, ListTurnsResponse, SearchEventsArgs,
    SearchEventsResponse,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Cursor {
    pub offset: usize,
}

impl Cursor {
    pub fn encode(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, json.as_bytes())
    }

    pub fn decode(cursor: &str) -> Option<Self> {
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, cursor).ok()?;
        let json = String::from_utf8(bytes).ok()?;
        serde_json::from_str(&json).ok()
    }
}

pub struct SessionService<'a> {
    client: &'a Client,
}

impl<'a> SessionService<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    pub async fn search_events(
        &self,
        args: SearchEventsArgs,
    ) -> Result<SearchEventsResponse, String> {
        let limit = args.limit();
        let offset = args
            .cursor
            .as_ref()
            .and_then(|c| Cursor::decode(c))
            .map(|c| c.offset)
            .unwrap_or(0);

        let project_hash_filter = if let Some(root) = args.project_root {
            Some(agtrace_sdk::utils::project_hash_from_root(&root))
        } else {
            args.project_hash.map(|h| h.into())
        };

        let mut filter = if let Some(hash) = project_hash_filter {
            SessionFilter::project(hash).limit(1000)
        } else {
            SessionFilter::all().limit(1000)
        };

        if let Some(provider) = args.provider {
            filter = filter.provider(provider.as_str().to_string());
        }

        let sessions = if let Some(ref session_id) = args.session_id {
            let _handle = self
                .client
                .sessions()
                .get(session_id)
                .map_err(|e| format!("Session not found: {}", e))?;

            vec![agtrace_sdk::SessionSummary {
                id: session_id.clone(),
                provider: String::new(),
                project_hash: agtrace_sdk::types::ProjectHash::from(String::new()),
                project_root: None,
                start_ts: None,
                snippet: None,
                parent_session_id: None,
                spawned_by: None,
            }]
        } else {
            self.client
                .sessions()
                .list_without_refresh(filter)
                .map_err(|e| format!("Failed to list sessions: {}", e))?
        };

        let mut all_matches = Vec::new();

        for session_summary in sessions {
            let handle = match self.client.sessions().get(&session_summary.id) {
                Ok(h) => h,
                Err(_) => continue,
            };

            let session = match handle.assemble() {
                Ok(s) => s,
                Err(_) => continue,
            };

            let events = match handle.events() {
                Ok(e) => e,
                Err(_) => continue,
            };

            for (event_index, event) in events.iter().enumerate() {
                if let Some(ref event_type_filter) = args.event_type
                    && !event_type_filter.matches_payload(&event.payload)
                {
                    continue;
                }

                let event_json = match serde_json::to_string(&event.payload) {
                    Ok(j) => j,
                    Err(_) => continue,
                };

                if event_json.contains(&args.query) {
                    let (turn_index, step_index) = Self::find_event_location(&session, event_index);

                    let event_match = EventMatch::new(
                        session_summary.id.clone(),
                        event_index,
                        turn_index,
                        step_index,
                        event,
                    );
                    all_matches.push(event_match);
                }
            }
        }

        let fetch_limit = limit + 1;
        let mut matches: Vec<_> = all_matches
            .into_iter()
            .skip(offset)
            .take(fetch_limit)
            .collect();

        let has_more = matches.len() > limit;
        if has_more {
            matches.pop();
        }

        let next_cursor = if has_more {
            Some(
                Cursor {
                    offset: offset + limit,
                }
                .encode(),
            )
        } else {
            None
        };

        Ok(SearchEventsResponse {
            matches,
            next_cursor,
        })
    }

    pub async fn list_turns(&self, args: ListTurnsArgs) -> Result<ListTurnsResponse, String> {
        let handle = self
            .client
            .sessions()
            .get(&args.session_id)
            .map_err(|e| format!("Session not found: {}", e))?;

        let session = handle
            .assemble()
            .map_err(|e| format!("Failed to assemble session: {}", e))?;

        let limit = args.limit();
        let offset = args
            .cursor
            .as_ref()
            .and_then(|c| Cursor::decode(c))
            .map(|c| c.offset)
            .unwrap_or(0);

        let total_turns = session.turns.len();
        let remaining = total_turns.saturating_sub(offset);
        let has_more = remaining > limit;

        let next_cursor = if has_more {
            Some(
                Cursor {
                    offset: offset + limit,
                }
                .encode(),
            )
        } else {
            None
        };

        Ok(ListTurnsResponse::new(session, offset, limit, next_cursor))
    }

    pub async fn get_turns(&self, args: GetTurnsArgs) -> Result<GetTurnsResponse, String> {
        let handle = self
            .client
            .sessions()
            .get(&args.session_id)
            .map_err(|e| format!("Session not found: {}", e))?;

        let session = handle
            .assemble()
            .map_err(|e| format!("Failed to assemble session: {}", e))?;

        GetTurnsResponse::new(session, &args)
    }

    fn find_event_location(
        session: &agtrace_sdk::types::AgentSession,
        event_index: usize,
    ) -> (usize, usize) {
        let mut current_event_idx = 0;

        for (turn_idx, turn) in session.turns.iter().enumerate() {
            for (step_idx, step) in turn.steps.iter().enumerate() {
                let step_event_count = Self::count_step_events(step);

                if current_event_idx + step_event_count > event_index {
                    return (turn_idx, step_idx);
                }

                current_event_idx += step_event_count;
            }
        }

        (0, 0)
    }

    fn count_step_events(step: &agtrace_sdk::types::AgentStep) -> usize {
        let mut count = 0;

        if step.reasoning.is_some() {
            count += 1;
        }

        count += step.tools.len() * 2;

        if step.message.is_some() {
            count += 1;
        }

        count
    }
}
