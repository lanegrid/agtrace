pub mod cmd {
    // Index commands
    pub const INDEX_UPDATE: &str = "agtrace index update";
    pub const INDEX_UPDATE_ALL_PROJECTS: &str = "agtrace index update --all-projects";
    pub const INDEX_REBUILD: &str = "agtrace index rebuild";

    // Session commands
    pub const SESSION_LIST: &str = "agtrace session list";
    pub const SESSION_SHOW_COMPACT: &str = "agtrace session show <id> --style compact";

    // Provider commands
    pub const PROVIDER_LIST: &str = "agtrace provider list";
    pub const PROVIDER_DETECT: &str = "agtrace provider detect";
    pub const PROVIDER_SET: &str = "agtrace provider set <name> --log-root <PATH> --enable";

    // Doctor commands
    pub const DOCTOR_RUN: &str = "agtrace doctor run";
    pub const DOCTOR_CHECK: &str = "agtrace doctor check <file-path>";
    pub const DOCTOR_INSPECT: &str = "agtrace doctor inspect <file-path>";

    // Init commands
    pub const INIT: &str = "agtrace init";
    pub const INIT_REFRESH: &str = "agtrace init --refresh";
    pub const INIT_ALL_PROJECTS: &str = "agtrace init --all-projects";

    // Shorthand aliases
    pub const LIST: &str = "agtrace list";
    pub const LIST_ALL_PROJECTS: &str = "agtrace session list --all-projects";
}

pub mod fmt {
    pub fn doctor_inspect(file_path: &str) -> String {
        format!("agtrace doctor inspect {}", file_path)
    }

    pub fn doctor_inspect_lines(file_path: &str, lines: usize) -> String {
        format!("agtrace doctor inspect {} --lines {}", file_path, lines)
    }

    pub fn doctor_inspect_json(file_path: &str) -> String {
        format!("agtrace doctor inspect {} --format json", file_path)
    }

    pub fn session_list_limit(limit: usize) -> String {
        format!("agtrace session list --limit {}", limit)
    }
}
