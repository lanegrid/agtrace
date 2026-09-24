//! Claude Code provider (≥ 2.1.24x transcripts).

pub(crate) mod content;
pub mod decoder;
pub mod discovery;
pub mod header;
pub mod mapper;
pub mod models;
pub mod provider;
pub(crate) mod records;
pub mod sidecar;
pub(crate) mod tags;
pub mod tool_mapping;
pub mod tools;

pub use self::decoder::ClaudeDecoder;
pub use self::discovery::{
    agent_files_in, encode_project_dir, is_agent_file_path, normalize_claude_file, project_dirs,
    read_snippet,
};
pub use self::header::read_claude_header;
pub use self::mapper::ClaudeToolMapper;
pub use self::provider::ClaudeProvider;
pub use self::sidecar::{
    ClaudeProcessEntry, ClaudeProcessStatus, ClaudeSubagentMeta, ClaudeTeamConfig,
    ClaudeTeamMemberConfig, read_process_entry, read_subagent_meta, read_team_config,
    session_registry_paths, team_config_paths,
};
pub use self::tool_mapping::{mcp_server_name, mcp_tool_name, parse_mcp_name};
