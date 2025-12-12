use agtrace::cli::{
    Cli, Commands, DoctorCommand, IndexCommand, LabCommand, ProviderCommand, SessionCommand,
};
use clap::Parser;

#[test]
fn test_scan_maps_to_index_update() {
    let legacy = Cli::try_parse_from(["agtrace", "scan"]).unwrap();
    let new = Cli::try_parse_from(["agtrace", "index", "update"]).unwrap();

    match (&legacy.command, &new.command) {
        (Commands::Scan { provider, force, verbose }, Commands::Index { command }) => {
            assert_eq!(*provider, "all");
            assert!(!force);
            assert!(!verbose);

            if let IndexCommand::Update { provider: new_provider, verbose: new_verbose } = command {
                assert_eq!(new_provider, "all");
                assert!(!new_verbose);
            } else {
                panic!("Expected IndexCommand::Update");
            }
        }
        _ => panic!("Unexpected command variants"),
    }
}

#[test]
fn test_scan_force_maps_to_index_rebuild() {
    let legacy = Cli::try_parse_from(["agtrace", "scan", "--force"]).unwrap();
    let new = Cli::try_parse_from(["agtrace", "index", "rebuild"]).unwrap();

    match (&legacy.command, &new.command) {
        (Commands::Scan { provider, force, verbose }, Commands::Index { command }) => {
            assert_eq!(*provider, "all");
            assert!(*force);
            assert!(!verbose);

            if let IndexCommand::Rebuild { provider: new_provider, verbose: new_verbose } = command {
                assert_eq!(new_provider, "all");
                assert!(!new_verbose);
            } else {
                panic!("Expected IndexCommand::Rebuild");
            }
        }
        _ => panic!("Unexpected command variants"),
    }
}

#[test]
fn test_list_maps_to_session_list() {
    let legacy = Cli::try_parse_from(["agtrace", "list", "--limit", "10"]).unwrap();
    let new = Cli::try_parse_from(["agtrace", "session", "list", "--limit", "10"]).unwrap();

    match (&legacy.command, &new.command) {
        (Commands::List { project_hash, source, limit, since, until }, Commands::Session { command }) => {
            assert!(project_hash.is_none());
            assert!(source.is_none());
            assert_eq!(*limit, 10);
            assert!(since.is_none());
            assert!(until.is_none());

            if let SessionCommand::List {
                project_hash: new_project_hash,
                source: new_source,
                limit: new_limit,
                since: new_since,
                until: new_until,
            } = command {
                assert!(new_project_hash.is_none());
                assert!(new_source.is_none());
                assert_eq!(new_limit, &10);
                assert!(new_since.is_none());
                assert!(new_until.is_none());
            } else {
                panic!("Expected SessionCommand::List");
            }
        }
        _ => panic!("Unexpected command variants"),
    }
}

#[test]
fn test_view_maps_to_session_show() {
    let legacy = Cli::try_parse_from(["agtrace", "view", "abc123"]).unwrap();
    let new = Cli::try_parse_from(["agtrace", "session", "show", "abc123"]).unwrap();

    match (&legacy.command, &new.command) {
        (Commands::View { session_id, raw, json, timeline, hide, only, full, short, style },
         Commands::Session { command }) => {
            assert_eq!(session_id, "abc123");
            assert!(!raw);
            assert!(!json);
            assert!(!timeline);
            assert!(hide.is_none());
            assert!(only.is_none());
            assert!(!full);
            assert!(!short);
            assert_eq!(style, "timeline");

            if let SessionCommand::Show {
                session_id: new_session_id,
                raw: new_raw,
                json: new_json,
                timeline: new_timeline,
                hide: new_hide,
                only: new_only,
                full: new_full,
                short: new_short,
                style: new_style,
            } = command {
                assert_eq!(new_session_id, "abc123");
                assert!(!new_raw);
                assert!(!new_json);
                assert!(!new_timeline);
                assert!(new_hide.is_none());
                assert!(new_only.is_none());
                assert!(!new_full);
                assert!(!new_short);
                assert_eq!(new_style, "timeline");
            } else {
                panic!("Expected SessionCommand::Show");
            }
        }
        _ => panic!("Unexpected command variants"),
    }
}

#[test]
fn test_schema_maps_to_provider_schema() {
    let legacy = Cli::try_parse_from(["agtrace", "schema", "claude"]).unwrap();
    let new = Cli::try_parse_from(["agtrace", "provider", "schema", "claude"]).unwrap();

    match (&legacy.command, &new.command) {
        (Commands::Schema { provider, format }, Commands::Provider { command }) => {
            assert_eq!(provider, "claude");
            assert_eq!(format, "text");

            if let ProviderCommand::Schema {
                provider: new_provider,
                format: new_format
            } = command {
                assert_eq!(new_provider, "claude");
                assert_eq!(new_format, "text");
            } else {
                panic!("Expected ProviderCommand::Schema");
            }
        }
        _ => panic!("Unexpected command variants"),
    }
}

#[test]
fn test_diagnose_maps_to_doctor_run() {
    let legacy = Cli::try_parse_from(["agtrace", "diagnose", "--provider", "claude"]).unwrap();
    let new = Cli::try_parse_from(["agtrace", "doctor", "run", "--provider", "claude"]).unwrap();

    match (&legacy.command, &new.command) {
        (Commands::Diagnose { provider, verbose }, Commands::Doctor { command }) => {
            assert_eq!(provider, "claude");
            assert!(!verbose);

            if let DoctorCommand::Run {
                provider: new_provider,
                verbose: new_verbose
            } = command {
                assert_eq!(new_provider, "claude");
                assert!(!new_verbose);
            } else {
                panic!("Expected DoctorCommand::Run");
            }
        }
        _ => panic!("Unexpected command variants"),
    }
}

#[test]
fn test_inspect_maps_to_doctor_inspect() {
    let legacy = Cli::try_parse_from(["agtrace", "inspect", "/path/to/file.jsonl"]).unwrap();
    let new = Cli::try_parse_from(["agtrace", "doctor", "inspect", "/path/to/file.jsonl"]).unwrap();

    match (&legacy.command, &new.command) {
        (Commands::Inspect { file_path, lines, format }, Commands::Doctor { command }) => {
            assert_eq!(file_path, "/path/to/file.jsonl");
            assert_eq!(*lines, 50);
            assert_eq!(format, "raw");

            if let DoctorCommand::Inspect {
                file_path: new_file_path,
                lines: new_lines,
                format: new_format
            } = command {
                assert_eq!(new_file_path, "/path/to/file.jsonl");
                assert_eq!(new_lines, &50);
                assert_eq!(new_format, "raw");
            } else {
                panic!("Expected DoctorCommand::Inspect");
            }
        }
        _ => panic!("Unexpected command variants"),
    }
}

#[test]
fn test_validate_maps_to_doctor_check() {
    let legacy = Cli::try_parse_from(["agtrace", "validate", "/path/to/file.jsonl"]).unwrap();
    let new = Cli::try_parse_from(["agtrace", "doctor", "check", "/path/to/file.jsonl"]).unwrap();

    match (&legacy.command, &new.command) {
        (Commands::Validate { file_path, provider }, Commands::Doctor { command }) => {
            assert_eq!(file_path, "/path/to/file.jsonl");
            assert!(provider.is_none());

            if let DoctorCommand::Check {
                file_path: new_file_path,
                provider: new_provider
            } = command {
                assert_eq!(new_file_path, "/path/to/file.jsonl");
                assert!(new_provider.is_none());
            } else {
                panic!("Expected DoctorCommand::Check");
            }
        }
        _ => panic!("Unexpected command variants"),
    }
}

#[test]
fn test_analyze_maps_to_lab_analyze() {
    let legacy = Cli::try_parse_from(["agtrace", "analyze", "abc123"]).unwrap();
    let new = Cli::try_parse_from(["agtrace", "lab", "analyze", "abc123"]).unwrap();

    match (&legacy.command, &new.command) {
        (Commands::Analyze { session_id, detect, format }, Commands::Lab { command }) => {
            assert_eq!(session_id, "abc123");
            assert_eq!(detect, "all");
            assert_eq!(format, "plain");

            if let LabCommand::Analyze {
                session_id: new_session_id,
                detect: new_detect,
                format: new_format
            } = command {
                assert_eq!(new_session_id, "abc123");
                assert_eq!(new_detect, "all");
                assert_eq!(new_format, "plain");
            } else {
                panic!("Expected LabCommand::Analyze");
            }
        }
        _ => panic!("Unexpected command variants"),
    }
}

#[test]
fn test_export_maps_to_lab_export() {
    let legacy = Cli::try_parse_from(["agtrace", "export", "abc123"]).unwrap();
    let new = Cli::try_parse_from(["agtrace", "lab", "export", "abc123"]).unwrap();

    match (&legacy.command, &new.command) {
        (Commands::Export { session_id, output, format, strategy }, Commands::Lab { command }) => {
            assert_eq!(session_id, "abc123");
            assert!(output.is_none());
            assert_eq!(format, "jsonl");
            assert_eq!(strategy, "raw");

            if let LabCommand::Export {
                session_id: new_session_id,
                output: new_output,
                format: new_format,
                strategy: new_strategy
            } = command {
                assert_eq!(new_session_id, "abc123");
                assert!(new_output.is_none());
                assert_eq!(new_format, "jsonl");
                assert_eq!(new_strategy, "raw");
            } else {
                panic!("Expected LabCommand::Export");
            }
        }
        _ => panic!("Unexpected command variants"),
    }
}

#[test]
fn test_providers_maps_to_provider_list() {
    let legacy = Cli::try_parse_from(["agtrace", "providers"]).unwrap();
    let new = Cli::try_parse_from(["agtrace", "provider", "list"]).unwrap();

    match (&legacy.command, &new.command) {
        (Commands::Providers { command: legacy_cmd }, Commands::Provider { command: new_cmd }) => {
            assert!(legacy_cmd.is_none());

            if let ProviderCommand::List = new_cmd {
                // Expected
            } else {
                panic!("Expected ProviderCommand::List");
            }
        }
        _ => panic!("Unexpected command variants"),
    }
}

#[test]
fn test_global_options_preserved() {
    let legacy = Cli::try_parse_from([
        "agtrace",
        "--data-dir", "/custom/path",
        "--format", "json",
        "--project-root", "/project",
        "--all-projects",
        "list"
    ]).unwrap();

    let new = Cli::try_parse_from([
        "agtrace",
        "--data-dir", "/custom/path",
        "--format", "json",
        "--project-root", "/project",
        "--all-projects",
        "session", "list"
    ]).unwrap();

    assert_eq!(legacy.data_dir, "/custom/path");
    assert_eq!(legacy.format, "json");
    assert_eq!(legacy.project_root, Some("/project".to_string()));
    assert!(legacy.all_projects);

    assert_eq!(new.data_dir, "/custom/path");
    assert_eq!(new.format, "json");
    assert_eq!(new.project_root, Some("/project".to_string()));
    assert!(new.all_projects);
}

#[test]
fn test_deprecation_warning_with_json_format() {
    // When --format json is used, deprecation warnings should be suppressed
    let cli = Cli::try_parse_from([
        "agtrace",
        "--format", "json",
        "list"
    ]).unwrap();

    assert_eq!(cli.format, "json");

    // This test verifies the CLI parses correctly
    // The actual suppression is tested through integration tests
    match &cli.command {
        Commands::List { .. } => {
            // Expected legacy command
        }
        _ => panic!("Expected List command"),
    }
}

#[test]
fn test_deprecation_warning_suppressed_by_env_var() {
    // Set environment variable
    std::env::set_var("AGTRACE_NO_DEPRECATION_WARN", "1");

    let cli = Cli::try_parse_from([
        "agtrace",
        "list"
    ]).unwrap();

    // This test verifies the CLI parses correctly
    // The actual suppression is tested through integration tests
    match &cli.command {
        Commands::List { .. } => {
            // Expected legacy command
        }
        _ => panic!("Expected List command"),
    }

    // Clean up
    std::env::remove_var("AGTRACE_NO_DEPRECATION_WARN");
}
