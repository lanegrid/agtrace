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
    all_projects: bool,
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
                    let events = import_claude_directory(&provider_config.log_root, project_root_override, all_projects)?;
                    all_events.extend(events);
                }
                "codex" => {
                    let events = import_codex_directory(&provider_config.log_root, project_root_override, session_id_prefix, all_projects)?;
                    all_events.extend(events);
                }
                "gemini" => {
                    let events = import_gemini_directory(&provider_config.log_root, all_projects)?;
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
                all_events = import_claude_directory(&root_path, project_root_override, all_projects)?;
            }
            "codex" => {
                all_events = import_codex_directory(&root_path, project_root_override, session_id_prefix, all_projects)?;
            }
            "gemini" => {
                all_events = import_gemini_directory(&root_path, all_projects)?;
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
    all_projects: bool,
) -> Result<Vec<AgentEventV1>> {
    let mut all_events = Vec::new();

    // Determine the target project root and encoded directory name
    let (target_project_root, project_dir_name) = if all_projects {
        (None, None)
    } else if let Some(pr) = project_root_override {
        let root = PathBuf::from(pr);
        let dir_name = crate::utils::encode_claude_project_dir(&root);
        (Some(root), Some(dir_name))
    } else {
        let root = discover_project_root(None)?;
        let dir_name = crate::utils::encode_claude_project_dir(&root);
        (Some(root), Some(dir_name))
    };

    // Optimize search path: if not all_projects, only search the specific project directory
    let search_root = if let Some(ref dir_name) = project_dir_name {
        let project_specific_root = root.join(dir_name);
        if project_specific_root.exists() && project_specific_root.is_dir() {
            project_specific_root
        } else {
            // Project directory doesn't exist, return empty
            return Ok(all_events);
        }
    } else {
        root.clone()
    };

    for entry in WalkDir::new(&search_root)
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

        // When searching specific project directory, we can skip cwd verification
        // since the directory structure itself guarantees the project match.
        // However, for --all-projects mode, we still need to verify.
        if all_projects {
            if let Some(ref target_root) = target_project_root {
                if let Some(session_cwd) = claude::extract_cwd_from_claude_file(entry.path()) {
                    let session_cwd_path = Path::new(&session_cwd);
                    if !paths_equal(target_root, session_cwd_path) {
                        continue;
                    }
                } else {
                    eprintln!("Warning: Could not extract cwd from {}, skipping", entry.path().display());
                    continue;
                }
            }
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
    all_projects: bool,
) -> Result<Vec<AgentEventV1>> {
    let mut all_events = Vec::new();

    // If all_projects is true, skip project filtering
    let target_project_root = if all_projects {
        None
    } else if let Some(pr) = project_root_override {
        Some(PathBuf::from(pr))
    } else {
        Some(discover_project_root(None)?)
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

        // Skip empty/incomplete session files silently
        if codex::is_empty_codex_session(entry.path()) {
            continue;
        }

        // Skip project filtering if all_projects is true
        if let Some(ref target_root) = target_project_root {
            if let Some(session_cwd) = codex::extract_cwd_from_codex_file(entry.path()) {
                let session_cwd_path = Path::new(&session_cwd);
                if !paths_equal(target_root, session_cwd_path) {
                    continue;
                }
            } else {
                eprintln!("Warning: Could not extract cwd from {}, skipping", entry.path().display());
                continue;
            }
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

fn import_gemini_directory(root: &PathBuf, all_projects: bool) -> Result<Vec<AgentEventV1>> {
    let mut all_events = Vec::new();

    // Determine target project hash
    let target_project_hash = if all_projects {
        None
    } else {
        let project_root = discover_project_root(None)?;
        Some(crate::utils::project_hash_from_root(
            &project_root.to_string_lossy()
        ))
    };

    // Optimize: if not all_projects, check only the specific project hash directory
    if let Some(ref hash) = target_project_hash {
        let project_specific_dir = root.join(hash);
        if project_specific_dir.exists() && project_specific_dir.is_dir() {
            // Process only this specific project directory
            let logs_json_path = project_specific_dir.join("logs.json");
            if logs_json_path.exists() {
                process_gemini_project_directory(&project_specific_dir, &logs_json_path, &mut all_events)?;
            }
        }
        // If directory doesn't exist, just return empty results (no warning needed)
        return Ok(all_events);
    }

    // For --all-projects mode, iterate through all directories
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

        process_gemini_project_directory(&path, &logs_json_path, &mut all_events)?;
    }

    Ok(all_events)
}

fn process_gemini_project_directory(
    project_dir: &Path,
    logs_json_path: &Path,
    all_events: &mut Vec<AgentEventV1>,
) -> Result<()> {
    match gemini::normalize_gemini_file(logs_json_path) {
        Ok(events) => {
            all_events.extend(events);
        }
        Err(e) => {
            eprintln!("Warning: Failed to parse {}: {}", logs_json_path.display(), e);
        }
    }

    let chats_dir = project_dir.join("chats");
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

    Ok(())
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
                    let target_hash = crate::utils::project_hash_from_root(
                        &project_root.to_string_lossy()
                    );
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
                    let target_hash = crate::utils::project_hash_from_root(
                        &project_root.to_string_lossy()
                    );
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
                                let chat_filename = chat_path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("");

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
