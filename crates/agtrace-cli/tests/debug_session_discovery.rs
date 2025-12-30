//! Debug test to investigate session discovery failure

use agtrace_testing::TestWorld;
use agtrace_testing::providers::TestProvider;
use anyhow::Result;

#[test]
fn debug_session_file_creation() -> Result<()> {
    let mut world = TestWorld::new().with_project("my-project");

    // Step 1: Enable provider and capture log_root
    world.enable_provider(TestProvider::Claude)?;

    // Read config to see what log_root was configured
    let config_content = std::fs::read_to_string(world.data_dir().join("config.toml"))?;
    eprintln!("=== Config content ===");
    eprintln!("{}", config_content);

    // Step 2: Add session
    world.set_cwd("my-project");
    world.add_session(TestProvider::Claude, "test-session.jsonl")?;

    // Step 3: List all files in temp directory
    eprintln!("\n=== Temp directory structure ===");
    eprintln!("Temp root: {}", world.temp_dir().display());
    list_directory(world.temp_dir(), "")?;

    // Step 3.5: Check session file content
    eprintln!("\n=== Session file content (first 5 lines) ===");
    // Find the session file dynamically
    let claude_dir = world.temp_dir().join(".claude");
    let mut session_file_path: Option<std::path::PathBuf> = None;
    if let Ok(entries) = std::fs::read_dir(&claude_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir()
                && let Ok(files) = std::fs::read_dir(entry.path())
            {
                for file in files.flatten() {
                    if file.file_name() == "test-session.jsonl" {
                        session_file_path = Some(file.path());
                        break;
                    }
                }
            }
        }
    }

    if let Some(ref path) = session_file_path {
        eprintln!("Session file found at: {}", path.display());
        let content = std::fs::read_to_string(path)?;
        for (i, line) in content.lines().take(5).enumerate() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(cwd) = json.get("cwd") {
                    eprintln!("Line {}: cwd = {}", i + 1, cwd);
                    // Calculate hash from this cwd
                    if let Some(cwd_str) = cwd.as_str() {
                        let hash = agtrace_sdk::types::project_hash_from_root(cwd_str);
                        eprintln!("         Hash from cwd: {}", hash);
                    }
                }
                if let Some(session_id) = json.get("sessionId") {
                    eprintln!("Line {}: sessionId = {}", i + 1, session_id);
                }
            }
        }
    } else {
        eprintln!("Session file not found");
    }

    // Calculate expected project hash
    eprintln!("\n=== Project hash calculation ===");
    let current_cwd = world.cwd();
    eprintln!("Current cwd: {}", current_cwd.display());
    let expected_hash = agtrace_sdk::types::project_hash_from_root(&current_cwd.to_string_lossy());
    eprintln!("Expected project hash: {}", expected_hash);

    // Step 4: Run init and capture output
    eprintln!("\n=== Running init ===");
    eprintln!("CWD before init: {}", world.cwd().display());
    let init_result = world.run(&["init"])?;
    eprintln!("Init stdout:\n{}", init_result.stdout());
    eprintln!("Init stderr:\n{}", init_result.stderr());
    eprintln!("CWD after init: {}", world.cwd().display());

    // Step 5: Try to list sessions
    eprintln!("\n=== Listing sessions (without explicit project-root) ===");
    eprintln!("CWD before list: {}", world.cwd().display());
    let list_result = world.run(&["session", "list", "--format", "json"])?;
    eprintln!("List stdout:\n{}", list_result.stdout());

    // Step 5.5: Try with explicit --project-root
    eprintln!("\n=== Listing sessions (WITH explicit project-root) ===");
    let project_root_str = world.cwd().to_string_lossy();
    let list_result_explicit = world.run(&[
        "session",
        "list",
        "--project-root",
        &project_root_str,
        "--format",
        "json",
    ])?;
    eprintln!("List stdout:\n{}", list_result_explicit.stdout());

    let json = list_result.json()?;
    eprintln!("\n=== JSON response ===");
    eprintln!("{}", serde_json::to_string_pretty(&json)?);

    // Step 6: Run doctor to check if file can be parsed
    if let Some(path) = session_file_path {
        eprintln!("\n=== Running doctor check ===");
        let doctor_result = world.run(&[
            "doctor",
            "check",
            &path.to_string_lossy(),
            "--provider",
            "claude_code",
        ])?;
        eprintln!("Doctor stdout:\n{}", doctor_result.stdout());
        eprintln!("Doctor stderr:\n{}", doctor_result.stderr());
        eprintln!("Doctor success: {}", doctor_result.success());
    }

    Ok(())
}

fn list_directory(dir: &std::path::Path, indent: &str) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_string_lossy();

        if path.is_dir() {
            eprintln!("{}📁 {}/", indent, file_name);
            list_directory(&path, &format!("{}  ", indent))?;
        } else {
            let metadata = std::fs::metadata(&path)?;
            eprintln!("{}📄 {} ({} bytes)", indent, file_name, metadata.len());
        }
    }
    Ok(())
}
