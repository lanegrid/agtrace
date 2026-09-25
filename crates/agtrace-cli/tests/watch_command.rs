//! `agtrace watch --mode console` end to end against a writable copy of the
//! synthetic fixture workspace (`AGTRACE_CLAUDE_HOME` / `AGTRACE_CODEX_HOME`).

use agtrace_testing::live_fixture::{LiveFixture, PROJECT_ROOT};
use anyhow::Result;
use std::io::{BufRead, BufReader};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(20);

/// Run `agtrace watch --mode console <args>` until every `needle` was printed (or
/// the timeout); returns the lines read.
fn console_until(fx: &LiveFixture, args: &[&str], needles: &[&str]) -> Result<Vec<String>> {
    let data = tempfile::TempDir::new()?;
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_agtrace"));
    cmd.env("AGTRACE_CLAUDE_HOME", fx.claude_home())
        .env("AGTRACE_CODEX_HOME", fx.codex_home())
        .arg("--data-dir")
        .arg(data.path())
        .args(["--project-root", PROJECT_ROOT, "watch", "--mode", "console"])
        .args(args);
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().expect("piped stdout");

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    let deadline = Instant::now() + WAIT;
    let mut lines = Vec::new();
    while !needles
        .iter()
        .all(|n| lines.iter().any(|l: &String| l.contains(n)))
    {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(line) => lines.push(line),
            Err(_) => break,
        }
    }
    child.kill()?;
    child.wait()?;
    Ok(lines)
}

fn assert_printed(lines: &[String], needles: &[&str]) {
    for needle in needles {
        assert!(
            lines.iter().any(|l| l.contains(needle)),
            "missing {needle:?} in console output:\n{}",
            lines.join("\n")
        );
    }
}

#[test]
fn console_prints_the_project_agent_tree_and_feed() -> Result<()> {
    let fx = LiveFixture::new(chrono::Local::now().date_naive())?;
    let needles = [
        "watching project demo-project",
        // Tree: lead, teammate (badge T), subagent, Codex root and its child thread.
        "+ Demo project audit",
        "[T] audit-A",
        "codex /root",
        "judge",
        // Feed: teammate message, Codex encrypted NEW_TASK, plaintext FINAL_ANSWER.
        "audit-A → ",
        "NEW_TASK  [encrypted]",
        "FINAL_ANSWER",
        // Timeline rows of agents.
        "[audit-A]",
    ];
    let lines = console_until(&fx, &["--since", "1d"], &needles)?;
    assert_printed(&lines, &needles);

    // Each agent is announced once (later changes are `~` lines).
    let added = lines.iter().filter(|l| l.starts_with("+ ")).count();
    assert_eq!(added, 7, "{}", lines.join("\n"));
    Ok(())
}

#[test]
fn console_session_scope_watches_one_tree() -> Result<()> {
    let fx = LiveFixture::new(chrono::Local::now().date_naive())?;
    let root = format!("codex:{}", agtrace_testing::live_fixture::CODEX_ROOT);
    let lines = console_until(
        &fx,
        &["--session", &root],
        &["watching session 01900000", "codex /root", "judge"],
    )?;
    assert_printed(
        &lines,
        &["watching session 01900000", "codex /root", "judge"],
    );
    assert!(
        !lines.iter().any(|l| l.contains("audit-A")),
        "the Claude tree is out of scope:\n{}",
        lines.join("\n")
    );
    Ok(())
}

#[test]
fn tui_mode_requires_a_tty() -> Result<()> {
    let data = tempfile::TempDir::new()?;
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_agtrace"))
        .arg("--data-dir")
        .arg(data.path())
        .arg("watch")
        .output()?;
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("requires a TTY"));
    Ok(())
}
