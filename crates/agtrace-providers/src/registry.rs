use crate::{ClaudeProvider, CodexProvider, GeminiProvider, LogProvider};
use anyhow::{anyhow, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ProviderMetadata {
    pub name: &'static str,
    pub description: &'static str,
    pub default_log_path: &'static str,
}

const PROVIDERS: &[ProviderMetadata] = &[
    ProviderMetadata {
        name: "claude_code",
        description: "Claude Code IDE",
        default_log_path: "~/.claude/projects",
    },
    ProviderMetadata {
        name: "codex",
        description: "Codex CLI",
        default_log_path: "~/.codex/sessions",
    },
    ProviderMetadata {
        name: "gemini",
        description: "Gemini CLI",
        default_log_path: "~/.gemini/tmp",
    },
];

pub fn get_all_providers() -> &'static [ProviderMetadata] {
    PROVIDERS
}

pub fn get_provider_names() -> Vec<&'static str> {
    PROVIDERS.iter().map(|p| p.name).collect()
}

pub fn get_provider_metadata(name: &str) -> Option<&'static ProviderMetadata> {
    PROVIDERS.iter().find(|p| p.name == name)
}

pub fn create_provider(name: &str) -> Result<Box<dyn LogProvider>> {
    match name {
        "claude_code" | "claude" => Ok(Box::new(ClaudeProvider::new())),
        "codex" => Ok(Box::new(CodexProvider::new())),
        "gemini" => Ok(Box::new(GeminiProvider::new())),
        _ => Err(anyhow!("Unknown provider: {}", name)),
    }
}

pub fn create_all_providers() -> Vec<Box<dyn LogProvider>> {
    vec![
        Box::new(ClaudeProvider::new()),
        Box::new(CodexProvider::new()),
        Box::new(GeminiProvider::new()),
    ]
}

pub fn detect_provider_from_path(path: &str) -> Result<Box<dyn LogProvider>> {
    if path.contains(".claude/") {
        Ok(Box::new(ClaudeProvider::new()))
    } else if path.contains(".codex/") {
        Ok(Box::new(CodexProvider::new()))
    } else if path.contains(".gemini/") {
        Ok(Box::new(GeminiProvider::new()))
    } else {
        Err(anyhow!("Cannot detect provider from path: {}", path))
    }
}

pub fn expand_home_path(path: &str) -> Option<PathBuf> {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return Some(home.join(stripped));
        }
    }
    None
}

pub fn get_default_log_paths() -> Vec<(String, PathBuf)> {
    let mut paths = Vec::new();
    for provider in PROVIDERS {
        if let Some(expanded) = expand_home_path(provider.default_log_path) {
            paths.push((provider.name.to_string(), expanded));
        }
    }
    paths
}
