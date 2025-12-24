use crate::traits::ProviderAdapter;
use anyhow::{Result, anyhow};
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

// Legacy functions removed - use create_adapter, create_all_adapters, detect_adapter_from_path instead

pub fn expand_home_path(path: &str) -> Option<PathBuf> {
    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return Some(home.join(stripped));
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

// --- New trait-based adapter registry ---

/// Create a provider adapter by name (new trait-based architecture)
pub fn create_adapter(name: &str) -> Result<ProviderAdapter> {
    ProviderAdapter::from_name(name)
}

/// Create all provider adapters (new trait-based architecture)
pub fn create_all_adapters() -> Vec<ProviderAdapter> {
    vec![
        ProviderAdapter::claude(),
        ProviderAdapter::codex(),
        ProviderAdapter::gemini(),
    ]
}

/// Detect provider adapter from path (new trait-based architecture)
pub fn detect_adapter_from_path(path: &str) -> Result<ProviderAdapter> {
    if path.contains(".claude/") {
        Ok(ProviderAdapter::claude())
    } else if path.contains(".codex/") {
        Ok(ProviderAdapter::codex())
    } else if path.contains(".gemini/") {
        Ok(ProviderAdapter::gemini())
    } else {
        Err(anyhow!("Cannot detect provider from path: {}", path))
    }
}
