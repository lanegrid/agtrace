use super::*;
use agtrace_providers::decode_file;
use agtrace_types::EventPayload;
use std::fs::OpenOptions;
use std::io::Write;
use uuid::Uuid;

const SID: &str = "00000000-0000-4000-8000-0000000000aa";

fn claude() -> Arc<dyn Provider> {
    provider_for(ProviderId::ClaudeCode)
}

/// A minimal Claude user record (one complete JSON document, no newline).
fn user_line(n: u32, text: &str) -> String {
    format!(
        r#"{{"parentUuid":null,"isSidechain":false,"type":"user","message":{{"role":"user","content":"{text}"}},"uuid":"00000000-0000-4000-8000-{n:012}","timestamp":"2026-09-20T10:00:{s:02}.000Z","sessionId":"{SID}","cwd":"/work/demo-project","version":"2.1.250"}}"#,
        s = n % 60
    )
}

fn append(path: &Path, bytes: &[u8]) {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    f.write_all(bytes).unwrap();
    f.flush().unwrap();
}

fn user_texts(events: &[AgentEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::User(u) => Some(u.text.clone()),
            _ => None,
        })
        .collect()
}

fn session_file(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join(format!("{SID}.jsonl"))
}

fn cursor(path: &Path) -> FileCursor {
    FileCursor::new(claude(), path.to_path_buf(), DecodeOptions::default())
}

#[test]
fn line_written_in_two_chunks_is_decoded_once_complete() {
    let dir = tempfile::tempdir().unwrap();
    let path = session_file(&dir);
    let line = user_line(1, "hello");
    let (a, b) = line.split_at(line.len() / 2);

    append(&path, a.as_bytes());
    let mut cur = cursor(&path);
    assert!(matches!(cur.poll().unwrap(), TailOutcome::Unchanged));
    assert_eq!(cur.offset(), 0, "partial line must not be consumed");

    append(&path, format!("{b}\n").as_bytes());
    let out = cur.poll().unwrap();
    assert!(matches!(out, TailOutcome::Appended(_)));
    assert_eq!(user_texts(out.events()), vec!["hello"]);
    assert_eq!(cur.offset(), line.len() as u64 + 1);
    assert_eq!(cur.line(), 1);
    assert!(!cur.diagnostics().unwrap().has_errors());
}

#[test]
fn lines_appended_between_polls_are_delivered_in_order_without_repeats() {
    let dir = tempfile::tempdir().unwrap();
    let path = session_file(&dir);
    append(&path, format!("{}\n", user_line(1, "one")).as_bytes());

    let mut cur = cursor(&path);
    assert_eq!(user_texts(cur.poll().unwrap().events()), vec!["one"]);
    assert!(matches!(cur.poll().unwrap(), TailOutcome::Unchanged));

    let batch = format!(
        "{}\n{}\n{}\n",
        user_line(2, "two"),
        user_line(3, "three"),
        user_line(4, "four")
    );
    append(&path, batch.as_bytes());
    let out = cur.poll().unwrap();
    assert!(matches!(out, TailOutcome::Appended(_)));
    assert_eq!(user_texts(out.events()), vec!["two", "three", "four"]);
    let lines: Vec<u64> = out.events().iter().map(|e| e.origin.line).collect();
    assert_eq!(lines, vec![1, 2, 3]);
    assert!(matches!(cur.poll().unwrap(), TailOutcome::Unchanged));
}

#[test]
fn origin_offsets_match_file_positions() {
    let dir = tempfile::tempdir().unwrap();
    let path = session_file(&dir);
    let l1 = format!("{}\r\n", user_line(1, "crlf"));
    let l2 = format!("{}\n", user_line(2, "lf"));
    append(&path, l1.as_bytes());
    let mut cur = cursor(&path);
    cur.poll().unwrap();
    append(&path, l2.as_bytes());
    let out = cur.poll().unwrap();
    assert_eq!(out.events()[0].origin.byte_offset, l1.len() as u64);
    assert_eq!(out.events()[0].origin.line, 1);
}

#[test]
fn truncation_resets_and_rereads_from_start() {
    let dir = tempfile::tempdir().unwrap();
    let path = session_file(&dir);
    append(
        &path,
        format!("{}\n{}\n", user_line(1, "old-1"), user_line(2, "old-2")).as_bytes(),
    );
    let mut cur = cursor(&path);
    assert_eq!(cur.poll().unwrap().events().len(), 2);

    // Truncate in place (same inode) and write shorter content.
    std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    append(&path, format!("{}\n", user_line(3, "new")).as_bytes());

    let out = cur.poll().unwrap();
    assert!(matches!(out, TailOutcome::Reset(_)), "{out:?}");
    assert_eq!(user_texts(out.events()), vec!["new"]);
    assert_eq!(out.events()[0].origin.line, 0);
    assert!(matches!(cur.poll().unwrap(), TailOutcome::Unchanged));
}

#[test]
fn identity_change_resets_even_when_file_grew() {
    let dir = tempfile::tempdir().unwrap();
    let path = session_file(&dir);
    append(&path, format!("{}\n", user_line(1, "first")).as_bytes());
    let mut cur = cursor(&path);
    assert_eq!(user_texts(cur.poll().unwrap().events()), vec!["first"]);

    // Replace the file by a different (larger) one via rename.
    let replacement = dir.path().join("replacement.tmp");
    append(
        &replacement,
        format!("{}\n{}\n", user_line(2, "a"), user_line(3, "b")).as_bytes(),
    );
    std::fs::rename(&replacement, &path).unwrap();

    let out = cur.poll().unwrap();
    assert!(matches!(out, TailOutcome::Reset(_)), "{out:?}");
    assert_eq!(user_texts(out.events()), vec!["a", "b"]);
}

#[test]
fn stale_partial_line_is_decoded_after_timeout() {
    let dir = tempfile::tempdir().unwrap();
    let path = session_file(&dir);
    append(
        &path,
        format!("{}\n{}", user_line(1, "done"), user_line(2, "crashed")).as_bytes(),
    );
    let mut cur = cursor(&path);
    let t0 = Instant::now();
    assert_eq!(user_texts(cur.poll_at(t0).unwrap().events()), vec!["done"]);

    // Not stale yet.
    let early = t0 + STALE_PARTIAL_AFTER - Duration::from_millis(1);
    assert!(matches!(
        cur.poll_at(early).unwrap(),
        TailOutcome::Unchanged
    ));

    let late = t0 + STALE_PARTIAL_AFTER;
    let out = cur.poll_at(late).unwrap();
    assert_eq!(user_texts(out.events()), vec!["crashed"]);
    assert_eq!(cur.line(), 2);
    assert!(matches!(cur.poll_at(late).unwrap(), TailOutcome::Unchanged));
}

#[test]
fn growing_partial_line_is_not_stale() {
    let dir = tempfile::tempdir().unwrap();
    let path = session_file(&dir);
    let line = user_line(1, "slow");
    let (a, b) = line.split_at(10);
    append(&path, a.as_bytes());
    let mut cur = cursor(&path);
    let t0 = Instant::now();
    cur.poll_at(t0).unwrap();
    // It grows just before the timeout: the timer restarts.
    append(&path, &b.as_bytes()[..5]);
    let t1 = t0 + STALE_PARTIAL_AFTER - Duration::from_millis(1);
    assert!(matches!(cur.poll_at(t1).unwrap(), TailOutcome::Unchanged));
    let t2 = t0 + STALE_PARTIAL_AFTER + Duration::from_millis(1);
    assert!(matches!(cur.poll_at(t2).unwrap(), TailOutcome::Unchanged));
    append(&path, format!("{}\n", &b[5..]).as_bytes());
    assert_eq!(user_texts(cur.poll_at(t2).unwrap().events()), vec!["slow"]);
}

#[test]
fn stale_invalid_partial_is_counted_not_fatal() {
    let dir = tempfile::tempdir().unwrap();
    let path = session_file(&dir);
    append(
        &path,
        format!("{}\n{{\"type\":\"user\",\"mess", user_line(1, "ok")).as_bytes(),
    );
    let mut cur = cursor(&path).with_stale_after(Duration::ZERO);
    let out = cur.poll().unwrap();
    assert_eq!(user_texts(out.events()), vec!["ok"]);
    // Second poll: nothing new, partial is stale ⇒ decoded and counted as invalid.
    assert!(matches!(cur.poll().unwrap(), TailOutcome::Unchanged));
    assert_eq!(cur.diagnostics().unwrap().invalid_json, 1);
    assert_eq!(cur.line(), 2);
}

#[test]
fn empty_file_waits_for_header() {
    let dir = tempfile::tempdir().unwrap();
    let path = session_file(&dir);
    append(&path, b"");
    let mut cur = cursor(&path);
    assert!(matches!(cur.poll().unwrap(), TailOutcome::Unchanged));
    assert!(cur.header().is_none());
    append(&path, format!("{}\n", user_line(1, "hi")).as_bytes());
    assert_eq!(user_texts(cur.poll().unwrap().events()), vec!["hi"]);
    assert!(cur.header().is_some());
}

#[test]
fn missing_file_is_an_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut cur = cursor(&dir.path().join("nope.jsonl"));
    let err = cur.poll().unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
}

// --- decode(file) == tail(file) equivalence on every fixture -------------------

fn fixture_files() -> Vec<(ProviderId, PathBuf)> {
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let roots = [
        crates.join("agtrace-testing/fixtures"),
        crates.join("agtrace-providers/tests/samples"),
    ];
    let mut files = Vec::new();
    for root in roots {
        for entry in walkdir::WalkDir::new(&root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !entry.file_type().is_file()
                || path.extension().and_then(|e| e.to_str()) != Some("jsonl")
            {
                continue;
            }
            let s = path.to_string_lossy();
            let name = path.file_name().unwrap().to_string_lossy();
            let provider = if s.contains("/codex/")
                || name.starts_with("rollout")
                || name.starts_with("codex")
            {
                ProviderId::Codex
            } else {
                ProviderId::ClaudeCode
            };
            files.push((provider, path.to_path_buf()));
        }
    }
    files.sort();
    assert!(
        files.len() >= 4,
        "fixture tree not found: {}",
        crates.display()
    );
    files
}

fn as_json(events: &[AgentEvent]) -> Vec<serde_json::Value> {
    events
        .iter()
        .map(|e| serde_json::to_value(e).unwrap())
        .collect()
}

/// Fold a tail's outcomes into the current event list, honouring Reset and the
/// upsert rule (same id ⇒ later replaces earlier).
fn fold(acc: &mut Vec<AgentEvent>, out: TailOutcome) {
    let new = match out {
        TailOutcome::Unchanged => return,
        TailOutcome::Appended(e) => e,
        TailOutcome::Reset(e) => {
            acc.clear();
            e
        }
    };
    for ev in new {
        match acc.iter().position(|x| x.id == ev.id) {
            Some(i) => acc[i] = ev,
            None => acc.push(ev),
        }
    }
}

fn batch(provider: &dyn Provider, path: &Path, opts: DecodeOptions) -> Vec<AgentEvent> {
    let (_, events, _) = decode_file(provider, path, opts).unwrap();
    // decode_file returns every emission; apply the same upsert rule.
    let mut acc = Vec::new();
    fold(&mut acc, TailOutcome::Appended(events));
    acc
}

#[test]
fn tail_from_zero_equals_decode_file_on_fixtures() {
    for (id, path) in fixture_files() {
        let provider = provider_for(id);
        let (_, batch_events, batch_diag) =
            decode_file(provider.as_ref(), &path, DecodeOptions::default()).unwrap();

        let mut cur = FileCursor::new(provider.clone(), path.clone(), DecodeOptions::default());
        let out = cur.poll().unwrap();
        let ids: Vec<Uuid> = out.events().iter().map(|e| e.id).collect();
        let batch_ids: Vec<Uuid> = batch_events.iter().map(|e| e.id).collect();
        assert_eq!(ids, batch_ids, "{}", path.display());
        assert_eq!(
            as_json(out.events()),
            as_json(&batch_events),
            "{}",
            path.display()
        );
        assert_eq!(
            serde_json::to_value(cur.diagnostics().unwrap()).unwrap(),
            serde_json::to_value(&batch_diag).unwrap(),
            "{}",
            path.display()
        );
        assert!(matches!(cur.poll().unwrap(), TailOutcome::Unchanged));
    }
}

#[test]
fn tail_in_arbitrary_chunks_equals_decode_file_on_fixtures() {
    let fallback = DateTime::parse_from_rfc3339("2026-09-20T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let opts = DecodeOptions {
        fallback_timestamp: Some(fallback),
    };
    for (id, src) in fixture_files() {
        let provider = provider_for(id);
        let bytes = std::fs::read(&src).unwrap();
        for chunk in [1usize, 7, 113, 4096] {
            // One poll per chunk: keep the tiny chunk sizes to small files so the
            // test stays fast as the fixture corpus grows.
            if (chunk == 1 && bytes.len() > 4 * 1024) || (chunk == 7 && bytes.len() > 16 * 1024) {
                continue;
            }
            let dir = tempfile::tempdir().unwrap();
            // Keep the file name: headers derive ids from it.
            let rel = src.file_name().unwrap();
            let path = if src.to_string_lossy().contains("/subagents/") {
                let sid = src
                    .parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.file_name())
                    .unwrap();
                let d = dir.path().join(sid).join("subagents");
                std::fs::create_dir_all(&d).unwrap();
                d.join(rel)
            } else {
                dir.path().join(rel)
            };
            append(&path, b"");

            let mut cur = FileCursor::new(provider.clone(), path.clone(), opts.clone());
            let mut tailed = Vec::new();
            for piece in bytes.chunks(chunk) {
                append(&path, piece);
                fold(&mut tailed, cur.poll().unwrap());
            }
            let expected = batch(provider.as_ref(), &path, opts.clone());
            assert_eq!(
                as_json(&tailed),
                as_json(&expected),
                "{} (chunk {chunk})",
                src.display()
            );
            let (_, _, diag) = decode_file(provider.as_ref(), &path, opts.clone()).unwrap();
            assert_eq!(
                serde_json::to_value(cur.diagnostics().unwrap()).unwrap(),
                serde_json::to_value(&diag).unwrap(),
                "{} (chunk {chunk})",
                src.display()
            );
        }
    }
}

#[test]
fn header_refined_by_later_lines_restarts_with_final_header() {
    // File name differs from the session id and the first record carries no
    // sessionId: the first header read can only guess the id from the name.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("renamed.jsonl");
    append(&path, b"{\"type\":\"mode\",\"mode\":\"default\"}\n");
    let mut cur = cursor(&path);
    cur.poll().unwrap();
    assert_eq!(cur.header().unwrap().agent.id.as_str(), "claude:renamed");

    append(&path, format!("{}\n", user_line(1, "hi")).as_bytes());
    let out = cur.poll().unwrap();
    assert!(matches!(out, TailOutcome::Reset(_)), "{out:?}");
    let expected = format!("claude:{SID}");
    assert_eq!(cur.header().unwrap().agent.id.as_str(), expected);
    assert!(out.events().iter().all(|e| e.agent.as_str() == expected));
    assert_eq!(user_texts(out.events()), vec!["hi"]);
    assert!(matches!(cur.poll().unwrap(), TailOutcome::Unchanged));
}
