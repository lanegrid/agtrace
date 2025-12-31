use agtrace_types::*;

#[test]
fn test_truncate() {
    let short = "short";
    assert_eq!(truncate(short, 10), "short");

    let long = "this is a very long string";
    let truncated = truncate(long, 10);
    assert!(truncated.len() < long.len());
    assert!(truncated.contains("...(truncated)"));
}

#[test]
fn test_is_64_char_hex() {
    assert!(is_64_char_hex(
        "a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890"
    ));
    assert!(!is_64_char_hex("not_hex"));
    assert!(!is_64_char_hex("a1b2c3")); // too short
}
