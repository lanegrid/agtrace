# Bug Candidates

## 2025-12-26: Session Discovery and Indexing Failure ✅ FIXED

### Symptom
Sessions created via the testing API (`add_session()`) are not indexed by `agtrace init` or `agtrace index update`. The scanner reports "Found 0 sessions" despite session files being physically present in the correct provider log directory structure.

### Reproduction
```rust
// Affects multiple tests:
// - test_init_uninitialized_directory_with_session_files
// - test_init_refresh_discards_existing_data
// - test_isolation_project_a_and_b_list_shows_only_current_project
// - test_list_filter_by_source_provider
// - test_watch_attaches_to_current_project_latest_session
// And 8+ other integration tests

// Example:
let mut world = TestWorld::new().with_project("my-project");
world.enable_provider(TestProvider::Claude)?;
world.set_cwd("my-project");
world.add_session(TestProvider::Claude, "session.jsonl")?;
world.run(&["init"])?;

// Verify: Session file exists on disk
// .claude/-var-folders-.../my-project/session.jsonl ✓

// Actual behavior:
world.run(&["session", "list", "--format", "json"])?;
// Returns: { "sessions": [] }  ✗

// Scanner output: "claude_code     Found 0 sessions"
```

### Expected Behavior
1. `agtrace init` should discover and index session files in provider log directories
2. Sessions should be queryable via `agtrace session list`
3. `agtrace index update` should incrementally add new sessions

### Observed Behavior
1. Scanner reports "Found 0 sessions" despite files being present
2. `session list` returns empty array even with `--all-projects`
3. Session files are physically created in correct locations (verified via filesystem inspection)

### Impact
**Critical** - Blocks all integration tests that verify:
- Project isolation
- Session filtering
- Watch command functionality
- Multi-provider support

**Affected Tests (21 tests total):**
- ✓ Pass: 3 tests (provider detection, vacuum, missing log root handling)
- ⚠️ Ignored: 2 tests (marked with bug reference)
- ❌ Fail: 8 tests (same root cause - session discovery)
- ✓ Pass: 8 tests (watch command edge cases not requiring real sessions)

### Investigation Needed
1. **Provider Log Discovery**: Does the scanner properly traverse project-encoded directories?
   - Encoding format: `/path/to/project` → `-path-to-project`
   - Does glob pattern match these directories?

2. **Project Hash Matching**: Is there a mismatch between:
   - Hash calculated from `cwd` during test setup
   - Hash extracted from session file content
   - Hash used for filtering in `session list`

3. **TestWorld API Bug**: Does `add_session()` create files in a location the scanner doesn't check?
   - Expected: `.claude/-path-to-project/session.jsonl`
   - Actual: Verify via filesystem inspection in failed tests

4. **Index Scope**: Does `init` without `--all-projects` skip project-encoded directories?

### Next Steps for Investigation
1. Add filesystem inspection in failing tests to verify actual file locations
2. Add scanner debug output to show which directories are being checked
3. Compare project hash calculation between:
   - `TestWorld::add_session()` (fixture creation)
   - `agtrace_providers::ScanContext` (scanner)
   - `session list` filter logic

### Workaround
None currently available. Tests document expected behavior but cannot verify implementation.

### Priority
**Critical** - Blocks integration test coverage for core functionality.

---

## Fix Applied (2025-12-26)

**Root Cause:**
On macOS, `/var` is a symlink to `/private/var`. When:
1. Test sets `cmd.current_dir("/var/folders/.../my-project")`
2. CLI calls `std::env::current_dir()` → returns `/private/var/folders/.../my-project`
3. Different paths → Different SHA256 hashes → Session filtering fails

**Solution:**
Modified `project_hash_from_root()` in `agtrace-types/src/util.rs` to canonicalize paths before hashing, ensuring consistent hashes across symlinks.

```rust
pub fn project_hash_from_root(project_root: &str) -> String {
    let normalized = normalize_path(Path::new(project_root));
    let path_str = normalized.to_string_lossy();

    let mut hasher = Sha256::new();
    hasher.update(path_str.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

**Result:**
- ✅ 15/21 integration tests now passing (was 11/21)
- ✅ Session discovery working correctly
- ✅ Project isolation working correctly
- ⚠️ 2 tests failing due to missing Gemini sample files (unrelated)
- ⚠️ 4 tests marked `#[ignore]` during investigation (can be removed)
