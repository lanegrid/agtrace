//! A writable copy of the synthetic v2026_09 fixture workspace for watcher tests.
//!
//! Layout of the copy (`<tmp>/claude` and `<tmp>/codex` act as `AGTRACE_CLAUDE_HOME`
//! / `AGTRACE_CODEX_HOME`):
//!
//! ```text
//! claude/projects/-work-demo-project/<lead>.jsonl, <teammate>.jsonl, <lead>/subagents/...
//! claude/teams/session-00000001/config.json
//! claude/sessions/4242.json
//! codex/sessions/<YYYY>/<MM>/<DD>/rollout-*.jsonl   (the given day, e.g. today)
//! codex/session_index.jsonl
//! ```

use chrono::NaiveDate;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Project cwd of every fixture agent.
pub const PROJECT_ROOT: &str = "/work/demo-project";
/// Claude lead session id.
pub const LEAD_SESSION: &str = "00000000-0000-4000-8000-000000000001";
/// Claude teammate session id (team `session-00000001`, name `audit-A`).
pub const TEAMMATE_SESSION: &str = "00000000-0000-4000-8000-000000000002";
/// Claude subagent / fork agent ids under the lead.
pub const SUBAGENT_ID: &str = "a0000000000000001";
pub const FORK_ID: &str = "a0000000000000002";
/// Codex threads.
pub const CODEX_ROOT: &str = "01900000-0000-7000-8000-000000000001";
pub const CODEX_CHILD: &str = "01900000-0000-7000-8000-000000000002";
pub const CODEX_FORK: &str = "01900000-0000-7000-8000-000000000003";

/// Directory of the committed fixture tree.
pub fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/v2026_09")
}

/// Writable copy of the fixture workspace.
pub struct LiveFixture {
    dir: TempDir,
    codex_day: NaiveDate,
}

impl LiveFixture {
    /// Copy the fixture tree; Codex rollouts are placed in the date dir of `codex_day`.
    pub fn new(codex_day: NaiveDate) -> std::io::Result<Self> {
        let fixture = Self {
            dir: TempDir::new()?,
            codex_day,
        };
        let src = fixture_root();
        copy_dir(&src.join("claude/home"), &fixture.claude_home())?;
        fs::create_dir_all(fixture.codex_day_dir())?;
        for entry in fs::read_dir(src.join("codex/home/sessions/2026/09/20"))? {
            let path = entry?.path();
            fs::copy(
                &path,
                fixture.codex_day_dir().join(path.file_name().unwrap()),
            )?;
        }
        let index = src.join("codex/home/session_index.jsonl");
        if index.exists() {
            fs::copy(index, fixture.codex_home().join("session_index.jsonl"))?;
        }
        Ok(fixture)
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// `AGTRACE_CLAUDE_HOME`
    pub fn claude_home(&self) -> PathBuf {
        self.dir.path().join("claude")
    }

    /// `AGTRACE_CODEX_HOME`
    pub fn codex_home(&self) -> PathBuf {
        self.dir.path().join("codex")
    }

    pub fn project_dir(&self) -> PathBuf {
        self.claude_home().join("projects/-work-demo-project")
    }

    pub fn lead_file(&self) -> PathBuf {
        self.project_dir().join(format!("{LEAD_SESSION}.jsonl"))
    }

    pub fn teammate_file(&self) -> PathBuf {
        self.project_dir().join(format!("{TEAMMATE_SESSION}.jsonl"))
    }

    pub fn subagent_file(&self, agent_id: &str) -> PathBuf {
        self.project_dir()
            .join(LEAD_SESSION)
            .join("subagents")
            .join(format!("agent-{agent_id}.jsonl"))
    }

    pub fn codex_day_dir(&self) -> PathBuf {
        self.codex_home()
            .join(format!("sessions/{}", self.codex_day.format("%Y/%m/%d")))
    }

    /// Rollout file of a Codex thread (by thread id suffix).
    pub fn codex_file(&self, thread: &str) -> PathBuf {
        fs::read_dir(self.codex_day_dir())
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .find(|p| p.to_string_lossy().ends_with(&format!("-{thread}.jsonl")))
            .unwrap_or_else(|| panic!("no rollout for {thread}"))
    }

    /// The (single) registry entry `sessions/<pid>.json`.
    pub fn registry_file(&self) -> PathBuf {
        fs::read_dir(self.claude_home().join("sessions"))
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|e| e == "json"))
            .unwrap_or_else(|| self.claude_home().join("sessions/4242.json"))
    }

    /// Point the registry entry at `pid` (e.g. `std::process::id()` for a live process).
    /// The file is renamed to `<pid>.json`.
    pub fn set_registry_pid(&self, pid: u32) -> std::io::Result<()> {
        let path = self.registry_file();
        let mut v: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        v["pid"] = serde_json::json!(pid);
        let new_path = self.claude_home().join(format!("sessions/{pid}.json"));
        fs::write(&new_path, serde_json::to_string_pretty(&v)?)?;
        if new_path != path {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Move a file out of the tree (returns where it went) so a test can "create" it
    /// later with [`restore`](Self::restore).
    pub fn stash(&self, path: &Path) -> std::io::Result<PathBuf> {
        let stash = self.dir.path().join("stash");
        fs::create_dir_all(&stash)?;
        let dest = stash.join(path.file_name().unwrap());
        fs::rename(path, &dest)?;
        Ok(dest)
    }

    /// Write a stashed file back to `path` (a fresh file with a new mtime).
    pub fn restore(&self, stashed: &Path, path: &Path) -> std::io::Result<()> {
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, fs::read(stashed)?)
    }

    /// Set the mtime of every agent / side-state file to `secs_ago` seconds in the past.
    pub fn age_all(&self, secs_ago: i64) -> std::io::Result<()> {
        let t = filetime::FileTime::from_unix_time(chrono::Utc::now().timestamp() - secs_ago, 0);
        for entry in walkdir::WalkDir::new(self.dir.path()) {
            let entry = entry.map_err(std::io::Error::other)?;
            if entry.file_type().is_file() {
                filetime::set_file_mtime(entry.path(), t)?;
            }
        }
        Ok(())
    }
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry.map_err(std::io::Error::other)?;
        let rel = entry.path().strip_prefix(src).unwrap();
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
