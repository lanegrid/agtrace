use assert_cmd::Command;

#[allow(deprecated)]
fn run_help(args: &[&str]) -> String {
    let mut cmd = Command::cargo_bin("agtrace").unwrap();
    let output = cmd.args(args).arg("--help").output().unwrap();
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn test_main_help() {
    let help = run_help(&[]);
    insta::assert_snapshot!("main_help", help);
}

#[test]
fn test_index_help() {
    let help = run_help(&["index"]);
    insta::assert_snapshot!("index_help", help);
}

#[test]
fn test_index_update_help() {
    let help = run_help(&["index", "update"]);
    insta::assert_snapshot!("index_update_help", help);
}

#[test]
fn test_index_rebuild_help() {
    let help = run_help(&["index", "rebuild"]);
    insta::assert_snapshot!("index_rebuild_help", help);
}

#[test]
fn test_index_vacuum_help() {
    let help = run_help(&["index", "vacuum"]);
    insta::assert_snapshot!("index_vacuum_help", help);
}

#[test]
fn test_session_help() {
    let help = run_help(&["session"]);
    insta::assert_snapshot!("session_help", help);
}

#[test]
fn test_session_list_help() {
    let help = run_help(&["session", "list"]);
    insta::assert_snapshot!("session_list_help", help);
}

#[test]
fn test_session_show_help() {
    let help = run_help(&["session", "show"]);
    insta::assert_snapshot!("session_show_help", help);
}

#[test]
fn test_provider_help() {
    let help = run_help(&["provider"]);
    insta::assert_snapshot!("provider_help", help);
}

#[test]
fn test_provider_list_help() {
    let help = run_help(&["provider", "list"]);
    insta::assert_snapshot!("provider_list_help", help);
}

#[test]
fn test_provider_detect_help() {
    let help = run_help(&["provider", "detect"]);
    insta::assert_snapshot!("provider_detect_help", help);
}

#[test]
fn test_provider_set_help() {
    let help = run_help(&["provider", "set"]);
    insta::assert_snapshot!("provider_set_help", help);
}

#[test]
fn test_provider_schema_help() {
    let help = run_help(&["provider", "schema"]);
    insta::assert_snapshot!("provider_schema_help", help);
}

#[test]
fn test_doctor_help() {
    let help = run_help(&["doctor"]);
    insta::assert_snapshot!("doctor_help", help);
}

#[test]
fn test_doctor_run_help() {
    let help = run_help(&["doctor", "run"]);
    insta::assert_snapshot!("doctor_run_help", help);
}

#[test]
fn test_doctor_inspect_help() {
    let help = run_help(&["doctor", "inspect"]);
    insta::assert_snapshot!("doctor_inspect_help", help);
}

#[test]
fn test_doctor_check_help() {
    let help = run_help(&["doctor", "check"]);
    insta::assert_snapshot!("doctor_check_help", help);
}

#[test]
fn test_project_help() {
    let help = run_help(&["project"]);
    insta::assert_snapshot!("project_help", help);
}

#[test]
fn test_project_list_help() {
    let help = run_help(&["project", "list"]);
    insta::assert_snapshot!("project_list_help", help);
}

#[test]
fn test_lab_help() {
    let help = run_help(&["lab"]);
    insta::assert_snapshot!("lab_help", help);
}

#[test]
fn test_lab_analyze_help() {
    let help = run_help(&["lab", "analyze"]);
    insta::assert_snapshot!("lab_analyze_help", help);
}

#[test]
fn test_lab_export_help() {
    let help = run_help(&["lab", "export"]);
    insta::assert_snapshot!("lab_export_help", help);
}

#[test]
fn test_init_help() {
    let help = run_help(&["init"]);
    insta::assert_snapshot!("init_help", help);
}

#[test]
fn test_pack_help() {
    let help = run_help(&["pack"]);
    insta::assert_snapshot!("pack_help", help);
}
