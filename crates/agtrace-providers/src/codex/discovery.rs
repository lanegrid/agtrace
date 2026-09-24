//! Codex discovery helpers (`session_index.jsonl` titles).

use std::collections::HashMap;
use std::path::Path;

/// File name of the Codex thread title index inside the Codex home directory.
pub const SESSION_INDEX_FILE: &str = "session_index.jsonl";

/// Read `session_index.jsonl` (`{id, thread_name, updated_at}` per line) into
/// `thread id -> title`. Later lines win (renames append a new line). Lenient: missing
/// files and bad lines are skipped.
pub fn read_session_index(path: &Path) -> HashMap<String, String> {
    let mut titles = HashMap::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return titles;
    };
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let id = v.get("id").and_then(|x| x.as_str());
        let name = v.get("thread_name").and_then(|x| x.as_str());
        if let (Some(id), Some(name)) = (id, name)
            && !name.is_empty()
        {
            titles.insert(id.to_string(), name.to_string());
        }
    }
    titles
}

#[cfg(test)]
mod session_index_tests {
    use super::*;

    #[test]
    fn latest_title_wins_and_bad_lines_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SESSION_INDEX_FILE);
        std::fs::write(
            &path,
            "{\"id\":\"t1\",\"thread_name\":\"first\",\"updated_at\":\"2026-09-20T10:00:00Z\"}\n\
             not json\n\
             {\"id\":\"t2\",\"thread_name\":\"other\"}\n\
             {\"id\":\"t1\",\"thread_name\":\"renamed\"}\n",
        )
        .unwrap();
        let titles = read_session_index(&path);
        assert_eq!(titles.get("t1").map(String::as_str), Some("renamed"));
        assert_eq!(titles.get("t2").map(String::as_str), Some("other"));
        assert!(read_session_index(&dir.path().join("missing.jsonl")).is_empty());
    }
}
