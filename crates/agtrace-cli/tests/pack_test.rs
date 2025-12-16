mod common;
use common::TestFixture;

#[test]
fn test_pack_template_generation() {
    let fixture = TestFixture::new();

    fixture
        .setup_provider("claude_code")
        .expect("Failed to setup provider");

    fixture
        .copy_sample_file("claude_session.jsonl", "session1.jsonl")
        .expect("Failed to copy sample file");

    fixture
        .copy_sample_file("claude_agent.jsonl", "session2.jsonl")
        .expect("Failed to copy sample file 2");

    fixture.index_update().expect("Failed to index");

    // Test 1: Compact template
    let mut cmd = fixture.command();
    let output = cmd
        .arg("pack")
        .arg("--template")
        .arg("compact")
        .arg("--all-projects")
        .output()
        .expect("Failed to run pack compact");

    assert!(
        output.status.success(),
        "pack compact failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let compact_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        !compact_output.is_empty(),
        "Compact pack should not be empty"
    );

    // Test 2: Diagnose template
    let mut cmd = fixture.command();
    let output = cmd
        .arg("pack")
        .arg("--template")
        .arg("diagnose")
        .arg("--all-projects")
        .output()
        .expect("Failed to run pack diagnose");

    assert!(
        output.status.success(),
        "pack diagnose failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let diagnose_output = String::from_utf8_lossy(&output.stdout);
    assert!(
        !diagnose_output.is_empty(),
        "Diagnose pack should not be empty"
    );

    // Test 3: Tools template
    let mut cmd = fixture.command();
    let output = cmd
        .arg("pack")
        .arg("--template")
        .arg("tools")
        .arg("--all-projects")
        .output()
        .expect("Failed to run pack tools");

    assert!(
        output.status.success(),
        "pack tools failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tools_output = String::from_utf8_lossy(&output.stdout);
    assert!(!tools_output.is_empty(), "Tools pack should not be empty");

    // Verify templates produce different output
    assert_ne!(
        compact_output.as_ref(),
        diagnose_output.as_ref(),
        "Compact and diagnose templates should produce different output"
    );
}
