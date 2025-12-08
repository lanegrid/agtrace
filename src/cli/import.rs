use crate::model::*;
use crate::normalize::{claude, codex, gemini};
use crate::utils::{discover_project_root, paths_equal};
use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn import_vendor_logs(
    source: &str,
    root: Option<&PathBuf>,
    project_root_override: Option<&str>,
    session_id_prefix: Option<&str>,
) -> Result<Vec<AgentEventV1>> {
    let mut all_events = Vec::new();

    if source == "all" {
        if root.is_some() {
            anyhow::bail!("Cannot specify --root when using --source=all");
        }

        let config = crate::config::Config::load()?;
        let enabled_providers = config.enabled_providers();

        if enabled_providers.is_empty() {
            eprintln!("Warning: No enabled providers found in config. Run 'agtrace providers detect' first.");
            return Ok(all_events);
        }

        for (provider_name, provider_config) in enabled_providers {
            println!("Importing from {} (log_root: {})", provider_name, provider_config.log_root.display());
            match provider_name.as_str() {
                "claude" => {
                    let events = import_claude_directory(&provider_config.log_root, project_root_override)?;
                    all_events.extend(events);
                }
                "codex" => {
                    let events = import_codex_directory(&provider_config.log_root, project_root_override, session_id_prefix)?;
                    all_events.extend(events);
                }
                "gemini" => {
                    let events = import_gemini_directory(&provider_config.log_root)?;
                    all_events.extend(events);
                }
                _ => {
                    eprintln!("Warning: Unknown provider '{}', skipping", provider_name);
                }
            }
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
            anyhow::bail!("Provider '{}' not found in config. Run 'agtrace providers detect' first.", source);
        }
    };

    if root_path.is_file() {
        match source {
            "claude" => {
                let events = claude::normalize_claude_file(&root_path, project_root_override)?;
                all_events.extend(events);
            }
            "codex" => {
                let filename = root_path.file_name()
                    .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?
                    .to_string_lossy();
                let session_id_base = if filename.ends_with(".jsonl") {
                    &filename[..filename.len() - 6]
                } else {
                    filename.as_ref()
                };

                let session_id = session_id_prefix
                    .map(|p| format!("{}{}", p, session_id_base))
                    .unwrap_or_else(|| session_id_base.to_string());

                let events = codex::normalize_codex_file(&root_path, &session_id, project_root_override)?;
                all_events.extend(events);
            }
            "gemini" => {
                let events = gemini::normalize_gemini_file(&root_path)?;
                all_events.extend(events);
            }
            _ => anyhow::bail!("Unknown source: {}", source),
        }
    } else if root_path.is_dir() {
        match source {
            "claude" => {
                all_events = import_claude_directory(&root_path, project_root_override)?;
            }
            "codex" => {
                all_events = import_codex_directory(&root_path, project_root_override, session_id_prefix)?;
            }
            "gemini" => {
                all_events = import_gemini_directory(&root_path)?;
            }
            _ => anyhow::bail!("Unknown source: {}", source),
        }
    } else {
        anyhow::bail!("Root path does not exist or is not accessible: {}", root_path.display());
    }

    Ok(all_events)
}

fn import_claude_directory(
    root: &PathBuf,
    project_root_override: Option<&str>,
) -> Result<Vec<AgentEventV1>> {
    let mut all_events = Vec::new();

    let target_project_root = if let Some(pr) = project_root_override {
        PathBuf::from(pr)
    } else {
        discover_project_root(None)?
    };

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        if !entry.path().extension().map_or(false, |e| e == "jsonl") {
            continue;
        }

        let _is_valid = is_valid_claude_file(entry.path());

        if let Some(session_cwd) = claude::extract_cwd_from_claude_file(entry.path()) {
            let session_cwd_path = Path::new(&session_cwd);
            if !paths_equal(&target_project_root, session_cwd_path) {
                continue;
            }
        } else {
            eprintln!("Warning: Could not extract cwd from {}, skipping", entry.path().display());
            continue;
        }

        match claude::normalize_claude_file(entry.path(), project_root_override) {
            Ok(events) => {
                all_events.extend(events);
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", entry.path().display(), e);
                continue;
            }
        }
    }

    Ok(all_events)
}

fn import_codex_directory(
    root: &PathBuf,
    project_root_override: Option<&str>,
    session_id_prefix: Option<&str>,
) -> Result<Vec<AgentEventV1>> {
    let mut all_events = Vec::new();

    let target_project_root = if let Some(pr) = project_root_override {
        PathBuf::from(pr)
    } else {
        discover_project_root(None)?
    };

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
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

        if let Some(session_cwd) = codex::extract_cwd_from_codex_file(entry.path()) {
            let session_cwd_path = Path::new(&session_cwd);
            if !paths_equal(&target_project_root, session_cwd_path) {
                continue;
            }
        } else {
            eprintln!("Warning: Could not extract cwd from {}, skipping", entry.path().display());
            continue;
        }

        let session_id_base = if filename.ends_with(".jsonl") {
            &filename[..filename.len() - 6]
        } else {
            filename.as_ref()
        };

        let session_id = session_id_prefix
            .map(|p| format!("{}{}", p, session_id_base))
            .unwrap_or_else(|| session_id_base.to_string());

        match codex::normalize_codex_file(entry.path(), &session_id, project_root_override) {
            Ok(events) => {
                all_events.extend(events);
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", entry.path().display(), e);
                continue;
            }
        }
    }

    Ok(all_events)
}

fn import_gemini_directory(root: &PathBuf) -> Result<Vec<AgentEventV1>> {
    let mut all_events = Vec::new();

    let project_root = discover_project_root(None)?;
    let target_project_hash = crate::utils::project_hash_from_root(
        &project_root.to_string_lossy()
    );

    let entries = std::fs::read_dir(root)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let dir_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if !is_64_char_hex(dir_name) {
            continue;
        }

        let logs_json_path = path.join("logs.json");
        if !logs_json_path.exists() {
            continue;
        }

        let mut session_project_hash = gemini::extract_project_hash_from_gemini_file(&logs_json_path);

        if session_project_hash.is_none() {
            let chats_dir = path.join("chats");
            if chats_dir.is_dir() {
                if let Ok(chat_entries) = std::fs::read_dir(&chats_dir) {
                    for chat_entry in chat_entries {
                        if let Ok(chat_entry) = chat_entry {
                            let chat_path = chat_entry.path();
                            if chat_path.is_file() && chat_path.extension().map_or(false, |e| e == "json") {
                                session_project_hash = gemini::extract_project_hash_from_gemini_file(&chat_path);
                                if session_project_hash.is_some() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(hash) = session_project_hash {
            if hash != target_project_hash {
                continue;
            }
        } else {
            eprintln!("Warning: Could not extract projectHash from {}, skipping", path.display());
            continue;
        }

        match gemini::normalize_gemini_file(&logs_json_path) {
            Ok(events) => {
                all_events.extend(events);
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", logs_json_path.display(), e);
            }
        }

        let chats_dir = path.join("chats");
        if chats_dir.is_dir() {
            if let Ok(chat_entries) = std::fs::read_dir(&chats_dir) {
                for chat_entry in chat_entries {
                    if let Ok(chat_entry) = chat_entry {
                        let chat_path = chat_entry.path();
                        let chat_filename = chat_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");

                        if chat_path.is_file()
                            && chat_filename.starts_with("session-")
                            && chat_filename.ends_with(".json")
                        {
                            match gemini::normalize_gemini_file(&chat_path) {
                                Ok(events) => {
                                    all_events.extend(events);
                                }
                                Err(e) => {
                                    eprintln!("Warning: Failed to parse {}: {}", chat_path.display(), e);
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(all_events)
}

pub fn count_claude_sessions(log_root: &PathBuf, project_root: &PathBuf) -> (usize, usize) {
    let mut total = 0;
    let mut matching = 0;

    for entry in WalkDir::new(log_root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        if !entry.path().extension().map_or(false, |e| e == "jsonl") {
            continue;
        }

        total += 1;

        if let Some(cwd) = claude::extract_cwd_from_claude_file(entry.path()) {
            let cwd_path = Path::new(&cwd);
            if paths_equal(project_root, cwd_path) {
                matching += 1;
            }
        }
    }

    (total, matching)
}

pub fn count_codex_sessions(log_root: &PathBuf, project_root: &PathBuf) -> (usize, usize) {
    let mut total = 0;
    let mut matching = 0;

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

        total += 1;

        if let Some(cwd) = codex::extract_cwd_from_codex_file(entry.path()) {
            let cwd_path = Path::new(&cwd);
            if paths_equal(project_root, cwd_path) {
                matching += 1;
            }
        }
    }

    (total, matching)
}

pub fn count_gemini_sessions(log_root: &PathBuf, target_project_hash: &str) -> (usize, usize) {
    let mut total = 0;
    let mut matching = 0;

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

                total += 1;

                if let Some(project_hash) = gemini::extract_project_hash_from_gemini_file(&logs_json_path) {
                    if project_hash == target_project_hash {
                        matching += 1;
                    }
                }
            }
        }
    }

    (total, matching)
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

fn is_valid_claude_file(path: &std::path::Path) -> Result<bool> {
    use std::io::{BufRead, BufReader};

    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    if let Some(Ok(first_line)) = reader.lines().next() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&first_line) {
            return Ok(json.get("type").is_some() && json.get("sessionId").is_some());
        }
    }

    Ok(false)
}
