use crate::core::importer::ImportService;
use crate::model::*;
use crate::providers::{claude, codex, gemini, ClaudeProvider, CodexProvider, GeminiProvider, ImportContext, LogProvider};
use anyhow::Result;
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn import_vendor_logs(
    source: &str,
    root: Option<&PathBuf>,
    project_root_override: Option<&str>,
    session_id_prefix: Option<&str>,
    all_projects: bool,
) -> Result<Vec<AgentEventV1>> {
    let context = ImportContext {
        project_root_override: project_root_override.map(|s| s.to_string()),
        session_id_prefix: session_id_prefix.map(|s| s.to_string()),
        all_projects,
    };

    if source == "all" {
        if root.is_some() {
            anyhow::bail!("Cannot specify --root when using --source=all");
        }

        let config = crate::config::Config::load()?;
        let enabled_providers = config.enabled_providers();

        if enabled_providers.is_empty() {
            eprintln!("Warning: No enabled providers found in config. Run 'agtrace providers detect' first.");
            return Ok(Vec::new());
        }

        let mut all_events = Vec::new();

        for (provider_name, provider_config) in enabled_providers {
            println!(
                "Importing from {} (log_root: {})",
                provider_name,
                provider_config.log_root.display()
            );

            let provider: Box<dyn LogProvider> = match provider_name.as_str() {
                "claude" => Box::new(ClaudeProvider::new()),
                "codex" => Box::new(CodexProvider::new()),
                "gemini" => Box::new(GeminiProvider::new()),
                _ => {
                    eprintln!("Warning: Unknown provider '{}', skipping", provider_name);
                    continue;
                }
            };

            let service = ImportService::new(vec![provider]);
            let events = service.import_path(&provider_config.log_root, &context)?;
            all_events.extend(events);
        }

        return Ok(all_events);
    }

    let root_path = if let Some(r) = root {
        r.clone()
    } else {
        let config = crate::config::Config::load()?;
        if let Some(provider_config) = config.providers.get(source) {
            provider_config.log_root.clone()
        } else {
            anyhow::bail!(
                "Provider '{}' not found in config. Run 'agtrace providers detect' first.",
                source
            );
        }
    };

    let provider: Box<dyn LogProvider> = match source {
        "claude" => Box::new(ClaudeProvider::new()),
        "codex" => Box::new(CodexProvider::new()),
        "gemini" => Box::new(GeminiProvider::new()),
        _ => anyhow::bail!("Unknown source: {}", source),
    };

    let service = ImportService::new(vec![provider]);
    service.import_path(&root_path, &context)
}

pub fn count_claude_sessions(log_root: &PathBuf, project_root: &PathBuf) -> (usize, usize) {
    use std::collections::HashSet;

    let mut total_sessions = HashSet::new();
    let mut matching_sessions = HashSet::new();

    for entry in WalkDir::new(log_root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        if !entry.path().extension().map_or(false, |e| e == "jsonl") {
            continue;
        }

        // Parse the file to extract session_id
        if let Ok(events) = claude::normalize_claude_file(entry.path(), None) {
            for event in events {
                if let Some(session_id) = &event.session_id {
                    total_sessions.insert(session_id.clone());

                    // Check if this session matches the project
                    let target_hash =
                        crate::utils::project_hash_from_root(&project_root.to_string_lossy());
                    if &event.project_hash == &target_hash {
                        matching_sessions.insert(session_id.clone());
                    }
                }
            }
        }
    }

    (total_sessions.len(), matching_sessions.len())
}

pub fn count_codex_sessions(log_root: &PathBuf, project_root: &PathBuf) -> (usize, usize) {
    use std::collections::HashSet;

    let mut total_sessions = HashSet::new();
    let mut matching_sessions = HashSet::new();

    for entry in WalkDir::new(log_root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        if !entry.path().extension().map_or(false, |e| e == "jsonl") {
            continue;
        }

        let filename = entry.file_name().to_string_lossy();
        if !filename.starts_with("rollout-") {
            continue;
        }

        // Skip empty/incomplete session files
        if codex::is_empty_codex_session(entry.path()) {
            continue;
        }

        // Use filename as fallback session_id
        let session_id_base = if filename.ends_with(".jsonl") {
            &filename[..filename.len() - 6]
        } else {
            filename.as_ref()
        };

        // Parse the file to extract session_id
        if let Ok(events) = codex::normalize_codex_file(entry.path(), session_id_base, None) {
            for event in events {
                if let Some(session_id) = &event.session_id {
                    total_sessions.insert(session_id.clone());

                    // Check if this session matches the project
                    let target_hash =
                        crate::utils::project_hash_from_root(&project_root.to_string_lossy());
                    if &event.project_hash == &target_hash {
                        matching_sessions.insert(session_id.clone());
                    }
                }
            }
        }
    }

    (total_sessions.len(), matching_sessions.len())
}

pub fn count_gemini_sessions(log_root: &PathBuf, target_project_hash: &str) -> (usize, usize) {
    use std::collections::HashSet;

    let mut total_sessions = HashSet::new();
    let mut matching_sessions = HashSet::new();

    if let Ok(entries) = std::fs::read_dir(log_root) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                if !path.is_dir() {
                    continue;
                }

                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if !is_64_char_hex(dir_name) {
                    continue;
                }

                let logs_json_path = path.join("logs.json");
                if !logs_json_path.exists() {
                    continue;
                }

                // Parse logs.json
                if let Ok(events) = gemini::normalize_gemini_file(&logs_json_path) {
                    for event in &events {
                        if let Some(session_id) = &event.session_id {
                            total_sessions.insert(session_id.clone());

                            if &event.project_hash == target_project_hash {
                                matching_sessions.insert(session_id.clone());
                            }
                        }
                    }
                }

                // Parse chats/*.json
                let chats_dir = path.join("chats");
                if chats_dir.is_dir() {
                    if let Ok(chat_entries) = std::fs::read_dir(&chats_dir) {
                        for chat_entry in chat_entries {
                            if let Ok(chat_entry) = chat_entry {
                                let chat_path = chat_entry.path();
                                let chat_filename =
                                    chat_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                                if chat_path.is_file()
                                    && chat_filename.starts_with("session-")
                                    && chat_filename.ends_with(".json")
                                {
                                    if let Ok(events) = gemini::normalize_gemini_file(&chat_path) {
                                        for event in &events {
                                            if let Some(session_id) = &event.session_id {
                                                total_sessions.insert(session_id.clone());

                                                if &event.project_hash == target_project_hash {
                                                    matching_sessions.insert(session_id.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    (total_sessions.len(), matching_sessions.len())
}

pub fn count_unique_sessions(events: &[AgentEventV1]) -> usize {
    let mut sessions = std::collections::HashSet::new();
    for event in events {
        if let Some(sid) = &event.session_id {
            sessions.insert(sid.clone());
        }
    }
    sessions.len()
}

fn is_64_char_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}
