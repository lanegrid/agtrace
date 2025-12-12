// DEPRECATED: This module is deprecated in favor of the SQLite-based index.
// Migration path: Use crate::index::Database instead.
// This file will be removed in Phase 4 (workspace split).

#![allow(deprecated)]

use crate::model::*;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[deprecated(
    since = "2.0.0",
    note = "Use crate::index::Database instead. This v1 file-based storage is replaced by SQLite pointer index."
)]
pub struct Storage {
    data_dir: PathBuf,
}

impl Storage {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    /// Helper: iterate over all session files
    fn iter_session_files(&self) -> impl Iterator<Item = PathBuf> {
        let projects_dir = self.data_dir.join("projects");
        WalkDir::new(projects_dir)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file() && e.path().extension().is_some_and(|ext| ext == "jsonl")
            })
            .map(|e| e.path().to_path_buf())
    }

    /// Save events to storage
    /// Events are grouped by project_hash and session_id
    /// Path: data_dir/projects/<project_hash>/sessions/<session_id>.jsonl
    pub fn save_events(&self, events: &[AgentEventV1]) -> Result<()> {
        // Group events by project_hash and session_id
        let mut grouped: HashMap<(String, String), Vec<&AgentEventV1>> = HashMap::new();

        for event in events {
            let session_id = event
                .session_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            let key = (event.project_hash.clone(), session_id);
            grouped.entry(key).or_default().push(event);
        }

        // Write each group to a file
        for ((project_hash, session_id), group) in grouped {
            let sessions_dir = self
                .data_dir
                .join("projects")
                .join(&project_hash)
                .join("sessions");
            fs::create_dir_all(&sessions_dir).with_context(|| {
                format!("Failed to create directory: {}", sessions_dir.display())
            })?;

            let file_path = sessions_dir.join(format!("{}.jsonl", session_id));

            // Read existing events if file exists
            let mut all_events: Vec<AgentEventV1> = if file_path.exists() {
                self.read_jsonl_file(&file_path)?
            } else {
                Vec::new()
            };

            // Collect existing event_ids for deduplication
            let mut existing_ids: HashSet<String> = all_events
                .iter()
                .filter_map(|e| e.event_id.clone())
                .collect();

            // Append new events with deduplication
            for event in group {
                if let Some(id) = &event.event_id {
                    // Skip if already exists (ensures idempotency)
                    if existing_ids.contains(id) {
                        continue;
                    }
                    // Add to set to prevent duplicates within the same batch
                    existing_ids.insert(id.clone());
                }

                // Add event if it has no ID or is a new ID
                all_events.push((*event).clone());
            }

            // Sort by timestamp to maintain chronological order
            all_events.sort_by(|a, b| a.ts.cmp(&b.ts));

            // Write all events
            self.write_jsonl_file(&file_path, &all_events)?;
        }

        Ok(())
    }

    /// Load all events for a specific session
    pub fn load_session_events(&self, session_id: &str) -> Result<Vec<AgentEventV1>> {
        let target_filename = format!("{}.jsonl", session_id);

        for path in self.iter_session_files() {
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                if filename == target_filename {
                    return self.read_jsonl_file(&path);
                }
            }
        }

        anyhow::bail!("Session not found: {}", session_id)
    }

    /// List all sessions with optional filters
    pub fn list_sessions(
        &self,
        project_hash: Option<&str>,
        source: Option<Source>,
        limit: Option<usize>,
        all_projects: bool,
    ) -> Result<Vec<SessionSummary>> {
        let mut summaries = Vec::new();

        for path in self.iter_session_files() {
            let events = self.read_jsonl_file(&path)?;

            if events.is_empty() {
                continue;
            }

            // Apply filters
            // If --project-hash is specified, filter by it (takes precedence over --all-projects)
            // If --all-projects is true and --project-hash is not specified, skip project filtering
            // Otherwise, filter by current project_hash if provided
            if let Some(ph) = project_hash {
                if events[0].project_hash != ph {
                    continue;
                }
            } else if !all_projects {
                // If all_projects is false and no project_hash specified,
                // this would normally filter by current project, but we don't have that context here.
                // The caller should provide the project_hash if all_projects is false.
            }

            if let Some(src) = source {
                if events[0].source != src {
                    continue;
                }
            }

            // Calculate summary
            let session_id = events[0]
                .session_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string());

            let start_ts = events.first().map(|e| e.ts.clone()).unwrap_or_default();
            let end_ts = events.last().map(|e| e.ts.clone()).unwrap_or_default();

            let user_message_count = events
                .iter()
                .filter(|e| e.event_type == EventType::UserMessage)
                .count();

            let tokens_input_total: u64 = events.iter().filter_map(|e| e.tokens_input).sum();

            let tokens_output_total: u64 = events.iter().filter_map(|e| e.tokens_output).sum();

            summaries.push(SessionSummary {
                session_id,
                source: events[0].source,
                project_hash: events[0].project_hash.clone(),
                start_ts,
                end_ts,
                event_count: events.len(),
                user_message_count,
                tokens_input_total,
                tokens_output_total,
            });
        }

        // Sort by start_ts (most recent first)
        summaries.sort_by(|a, b| b.start_ts.cmp(&a.start_ts));

        // Apply limit
        if let Some(lim) = limit {
            summaries.truncate(lim);
        }

        Ok(summaries)
    }

    /// Find events matching search criteria
    pub fn find_events(
        &self,
        session_id: Option<&str>,
        project_hash: Option<&str>,
        text_query: Option<&str>,
        event_type: Option<EventType>,
        limit: Option<usize>,
        all_projects: bool,
    ) -> Result<Vec<AgentEventV1>> {
        let mut results = Vec::new();

        for path in self.iter_session_files() {
            let events = self.read_jsonl_file(&path)?;

            for event in events {
                // Apply filters
                if let Some(sid) = session_id {
                    if event.session_id.as_deref() != Some(sid) {
                        continue;
                    }
                }

                // If --project-hash is specified, filter by it (takes precedence)
                // If --all-projects is true and --project-hash is not specified, skip project filtering
                if let Some(ph) = project_hash {
                    if event.project_hash != ph {
                        continue;
                    }
                } else if !all_projects {
                    // If all_projects is false and no project_hash specified,
                    // this would normally filter by current project, but we don't have that context here.
                    // The caller should provide the project_hash if all_projects is false.
                }

                if let Some(et) = event_type {
                    if event.event_type != et {
                        continue;
                    }
                }

                if let Some(query) = text_query {
                    if let Some(text) = &event.text {
                        if !text.to_lowercase().contains(&query.to_lowercase()) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                results.push(event);

                if let Some(lim) = limit {
                    if results.len() >= lim {
                        return Ok(results);
                    }
                }
            }
        }

        Ok(results)
    }

    // Helper methods
    fn read_jsonl_file(&self, path: &std::path::Path) -> Result<Vec<AgentEventV1>> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let mut events = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let event: AgentEventV1 = serde_json::from_str(line)
                .with_context(|| format!("Failed to parse JSON line: {}", line))?;
            events.push(event);
        }

        Ok(events)
    }

    fn write_jsonl_file(&self, path: &std::path::Path, events: &[AgentEventV1]) -> Result<()> {
        let mut lines = Vec::new();
        for event in events {
            let line = serde_json::to_string(event)?;
            lines.push(line);
        }

        fs::write(path, lines.join("\n") + "\n")
            .with_context(|| format!("Failed to write file: {}", path.display()))?;

        Ok(())
    }
}
