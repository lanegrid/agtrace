use crate::error::{Error, Result};
use crate::parser::{claude_code, codex};
use std::io::BufRead;
use std::path::PathBuf;

pub fn cmd_validate(agent: String, path: Option<PathBuf>) -> Result<()> {
    match agent.as_str() {
        "claude-code" => validate_claude(path),
        "codex" => validate_codex(path),
        other => Err(Error::UnknownAgent(other.to_string())),
    }
}

fn validate_claude(path: Option<PathBuf>) -> Result<()> {
    let dir = if let Some(path) = path {
        path
    } else {
        let home = home::home_dir()
            .ok_or_else(|| Error::Parse("Could not find home directory".to_string()))?;
        home.join(".claude").join("projects")
    };

    if !dir.exists() {
        return Err(Error::AgentDataNotFound(dir));
    }

    let mut files_checked = 0usize;
    let mut lines_checked = 0usize;

    for entry in walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().and_then(|s| s.to_str()) == Some("jsonl"))
    {
        let path = entry.path();
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);

        for (idx, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| {
                Error::Parse(format!(
                    "Failed to read line {} in {}: {}",
                    idx + 1,
                    path.display(),
                    e
                ))
            })?;
            if line.trim().is_empty() {
                continue;
            }

            serde_json::from_str::<claude_code::ClaudeCodeMessage>(&line).map_err(|e| {
                Error::Parse(format!(
                    "Failed to parse ClaudeCodeMessage from {} line {}: {}",
                    path.display(),
                    idx + 1,
                    e
                ))
            })?;

            lines_checked += 1;
        }

        files_checked += 1;
    }

    println!(
        "Validated Claude Code data at {} (files: {}, lines: {})",
        dir.display(),
        files_checked,
        lines_checked
    );

    Ok(())
}

fn validate_codex(path: Option<PathBuf>) -> Result<()> {
    let dir = if let Some(path) = path {
        path
    } else {
        let home = home::home_dir()
            .ok_or_else(|| Error::Parse("Could not find home directory".to_string()))?;
        home.join(".codex").join("sessions")
    };

    if !dir.exists() {
        return Err(Error::AgentDataNotFound(dir));
    }

    let mut files_checked = 0usize;
    let mut lines_checked = 0usize;

    for year_entry in std::fs::read_dir(&dir)? {
        let year_entry = year_entry?;
        let year_path = year_entry.path();
        if !year_path.is_dir() {
            continue;
        }

        for month_entry in std::fs::read_dir(&year_path)? {
            let month_entry = month_entry?;
            let month_path = month_entry.path();
            if !month_path.is_dir() {
                continue;
            }

            for day_entry in std::fs::read_dir(&month_path)? {
                let day_entry = day_entry?;
                let day_path = day_entry.path();
                if !day_path.is_dir() {
                    continue;
                }

                for file_entry in std::fs::read_dir(&day_path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();
                    if !file_path.is_file()
                        || file_path
                            .extension()
                            .and_then(|s| s.to_str())
                            != Some("jsonl")
                    {
                        continue;
                    }

                    let file = std::fs::File::open(&file_path)?;
                    let reader = std::io::BufReader::new(file);

                    for (idx, line) in reader.lines().enumerate() {
                        let line = line.map_err(|e| {
                            Error::Parse(format!(
                                "Failed to read line {} in {}: {}",
                                idx + 1,
                                file_path.display(),
                                e
                            ))
                        })?;
                        if line.trim().is_empty() {
                            continue;
                        }

                        serde_json::from_str::<codex::CodexEvent>(&line).map_err(|e| {
                            Error::Parse(format!(
                                "Failed to parse CodexEvent from {} line {}: {}",
                                file_path.display(),
                                idx + 1,
                                e
                            ))
                        })?;

                        lines_checked += 1;
                    }

                    files_checked += 1;
                }
            }
        }
    }

    println!(
        "Validated Codex data at {} (files: {}, lines: {})",
        dir.display(),
        files_checked,
        lines_checked
    );

    Ok(())
}

