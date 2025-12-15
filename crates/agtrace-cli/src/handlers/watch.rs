use agtrace_types::v2::{AgentEvent, EventPayload};
use anyhow::Result;
use chrono::Local;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::SystemTime;

/// Handle the watch command - auto-attach to latest session and stream formatted events
pub fn handle(log_root: &Path) -> Result<()> {
    println!("{} {}", "[👀 Watching]".bright_cyan(), log_root.display());

    // Track file offsets (path -> byte offset)
    let mut file_offsets: HashMap<PathBuf, u64> = HashMap::new();

    // Find and attach to the latest file
    let mut current_file = find_latest_log_file(log_root)?;
    if let Some(ref path) = current_file {
        let offset = std::fs::metadata(path)?.len();
        file_offsets.insert(path.clone(), offset);
        println!(
            "{}  {}\n",
            "✨ Attached to active session:".bright_green(),
            path.file_name().unwrap().to_string_lossy()
        );
    } else {
        println!("{}", "(Waiting for activity...)".dimmed());
    }

    // Set up file system watcher (recursive to handle subdirectories)
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, notify::Config::default())?;
    watcher.watch(log_root, RecursiveMode::Recursive)?;

    // Event loop
    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                if let Err(e) =
                    handle_fs_event(&event, &mut current_file, &mut file_offsets, log_root)
                {
                    eprintln!("Error handling event: {}", e);
                }
            }
            Ok(Err(e)) => eprintln!("Watch error: {:?}", e),
            Err(e) => {
                eprintln!("Channel error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Handle a file system event
fn handle_fs_event(
    event: &Event,
    current_file: &mut Option<PathBuf>,
    file_offsets: &mut HashMap<PathBuf, u64>,
    _log_root: &Path,
) -> Result<()> {
    match event.kind {
        EventKind::Create(_) => {
            for path in &event.paths {
                if is_log_file(path) {
                    // Switch to the new session
                    println!(
                        "\n{} {}\n",
                        "✨ New session detected:".bright_green(),
                        path.file_name().unwrap().to_string_lossy()
                    );
                    *current_file = Some(path.clone());
                    file_offsets.insert(path.clone(), 0);
                }
            }
        }
        EventKind::Modify(_) => {
            for path in &event.paths {
                if Some(path) == current_file.as_ref() {
                    // Read and display new content
                    let offset = *file_offsets.get(path).unwrap_or(&0);
                    let new_offset = process_new_lines(path, offset)?;
                    file_offsets.insert(path.clone(), new_offset);
                }
            }
        }
        _ => {}
    }

    Ok(())
}

/// Process new lines from a file starting at the given offset
fn process_new_lines(path: &Path, offset: u64) -> Result<u64> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let reader = BufReader::new(file);

    let mut new_offset = offset;
    for line in reader.lines() {
        let line = line?;
        new_offset += line.len() as u64 + 1; // +1 for newline

        // Parse and display event
        match serde_json::from_str::<AgentEvent>(&line) {
            Ok(event) => print_event(&event),
            Err(_) => {
                // Skip malformed lines silently (could be incomplete writes)
            }
        }
    }

    Ok(new_offset)
}

/// Print a formatted event to stdout
fn print_event(event: &AgentEvent) {
    let time = event.timestamp.with_timezone(&Local).format("%H:%M:%S");

    match &event.payload {
        EventPayload::User(payload) => {
            let text = truncate(&payload.text, 100);
            println!("{} {} \"{}\"", time.dimmed(), "👤 User:".bold(), text);
        }
        EventPayload::Reasoning(payload) => {
            let text = truncate(&payload.text, 50);
            println!(
                "{} {} {}",
                time.dimmed(),
                "🧠 Thnk:".dimmed(),
                text.dimmed()
            );
        }
        EventPayload::ToolCall(payload) => {
            let (icon, color_fn) = categorize_tool(&payload.name);
            let summary = format_tool_call(&payload.name, &payload.arguments);

            // Check for safety alerts
            let alert = check_safety_alert(&payload.arguments);

            let colored_name = color_fn(&payload.name);
            println!("{} {} {}: {}", time.dimmed(), icon, colored_name, summary);
            if let Some(warning) = alert {
                println!("             {} {}", "↳ ⚠️  ALERT:".red(), warning.red());
            }
        }
        EventPayload::ToolResult(payload) => {
            if payload.is_error {
                let output = truncate(&payload.output, 100);
                println!("{} {} {}", time.dimmed(), "❌ Fail:".red(), output.red());
            }
            // Success results are not shown (too noisy for MVP)
        }
        EventPayload::Message(payload) => {
            let text = truncate(&payload.text, 100);
            println!("{} {} {}", time.dimmed(), "💬 Msg:".cyan(), text);
        }
        EventPayload::TokenUsage(_) => {
            // Skip token usage (sidecar info, not relevant for stream)
        }
    }
}

/// Categorize a tool by name and return (icon, color_fn)
fn categorize_tool(name: &str) -> (&'static str, fn(&str) -> String) {
    let lower = name.to_lowercase();

    if lower.contains("read")
        || lower.contains("ls")
        || lower.contains("cat")
        || lower.contains("grep")
        || lower.contains("search")
        || lower.contains("view")
    {
        ("📖", |s: &str| s.cyan().to_string())
    } else if lower.contains("write") || lower.contains("edit") || lower.contains("replace") {
        ("🛠️", |s: &str| s.yellow().to_string())
    } else if lower.contains("run")
        || lower.contains("exec")
        || lower.contains("bash")
        || lower.contains("python")
        || lower.contains("test")
    {
        ("🧪", |s: &str| s.magenta().to_string())
    } else {
        ("🔧", |s: &str| s.white().to_string())
    }
}

/// Format tool call arguments into a concise summary
fn format_tool_call(_name: &str, args: &serde_json::Value) -> String {
    // Extract key arguments based on common patterns
    if let Some(obj) = args.as_object() {
        // Common argument names to look for
        if let Some(path) = obj.get("path").or_else(|| obj.get("file_path")) {
            if let Some(path_str) = path.as_str() {
                return format!("(\"{}\")", truncate(path_str, 60));
            }
        }
        if let Some(command) = obj.get("command") {
            if let Some(cmd_str) = command.as_str() {
                return format!("(\"{}\")", truncate(cmd_str, 60));
            }
        }
        if let Some(pattern) = obj.get("pattern") {
            if let Some(pat_str) = pattern.as_str() {
                return format!("(\"{}\")", truncate(pat_str, 60));
            }
        }
    }

    // Fallback: show first 40 chars of JSON
    let json = args.to_string();
    format!("({})", truncate(&json, 40))
}

/// Check for potentially dangerous operations
fn check_safety_alert(args: &serde_json::Value) -> Option<String> {
    if let Some(obj) = args.as_object() {
        // Check for path traversal
        for (_key, value) in obj.iter() {
            if let Some(s) = value.as_str() {
                if s.contains("..") {
                    return Some("Path contains '..' (outside access)".to_string());
                }
                if s.starts_with('/') && !s.starts_with("/Users/") && !s.starts_with("/home/") {
                    return Some("Absolute path outside user directory".to_string());
                }
            }
        }
    }
    None
}

/// Truncate text to max length with ellipsis
fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}

/// Find the most recently modified log file in the directory (recursive)
fn find_latest_log_file(dir: &Path) -> Result<Option<PathBuf>> {
    if !dir.exists() {
        return Ok(None);
    }

    let mut latest: Option<(PathBuf, SystemTime)> = None;

    // Use walkdir for recursive search
    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if path.is_file() && is_log_file(path) {
            if let Ok(metadata) = path.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if let Some((_, latest_time)) = &latest {
                        if modified > *latest_time {
                            latest = Some((path.to_path_buf(), modified));
                        }
                    } else {
                        latest = Some((path.to_path_buf(), modified));
                    }
                }
            }
        }
    }

    Ok(latest.map(|(path, _)| path))
}

/// Check if a file is a log file (JSONL)
fn is_log_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|ext| ext == "jsonl")
        .unwrap_or(false)
}
