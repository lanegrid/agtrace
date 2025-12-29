//! Fixtures for sample data generation and placement.
//!
//! Provides utilities to:
//! - Copy sample files to test environments
//! - Modify sample data for isolation (e.g., cwd, sessionId)
//! - Generate provider-specific directory structures

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Sample file manager for test data.
pub struct SampleFiles {
    samples_dir: PathBuf,
}

impl Default for SampleFiles {
    fn default() -> Self {
        Self::new()
    }
}

impl SampleFiles {
    /// Create a new sample file manager.
    ///
    /// Assumes samples are in `crates/agtrace-providers/tests/samples/`.
    pub fn new() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let samples_dir = manifest_dir
            .parent()
            .unwrap()
            .join("agtrace-providers/tests/samples");

        Self { samples_dir }
    }

    /// Copy a sample file to a destination.
    pub fn copy_to(&self, sample_name: &str, dest: &Path) -> Result<()> {
        let source = self.samples_dir.join(sample_name);
        fs::copy(source, dest)?;
        Ok(())
    }

    /// Copy a sample file to a Claude-encoded project directory.
    ///
    /// Claude encodes project paths like: `/Users/foo/bar` -> `-Users-foo-bar`
    pub fn copy_to_project(
        &self,
        sample_name: &str,
        dest_name: &str,
        project_dir: &str,
        log_root: &Path,
    ) -> Result<()> {
        let source = self.samples_dir.join(sample_name);

        // Encode project directory (Claude format)
        let encoded = project_dir
            .replace(['/', '.'], "-")
            .trim_start_matches('-')
            .to_string();
        let encoded_dir = format!("-{}", encoded);

        let project_log_dir = log_root.join(encoded_dir);
        fs::create_dir_all(&project_log_dir)?;

        let dest = project_log_dir.join(dest_name);
        fs::copy(source, dest)?;
        Ok(())
    }

    /// Copy a sample file with cwd and sessionId replacement.
    ///
    /// This creates isolated test sessions by:
    /// 1. Replacing the embedded `cwd` field with `target_project_dir` (canonicalized)
    /// 2. Generating a unique `sessionId` based on project dir + filename
    /// 3. Using provider-specific directory encoding via the provider adapter
    pub fn copy_to_project_with_cwd(
        &self,
        sample_name: &str,
        dest_name: &str,
        target_project_dir: &str,
        log_root: &Path,
        provider_adapter: &agtrace_providers::ProviderAdapter,
    ) -> Result<()> {
        let source = self.samples_dir.join(sample_name);

        // Canonicalize target_project_dir to match project_hash_from_root behavior
        let canonical_project_dir = Path::new(target_project_dir)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(target_project_dir).to_path_buf());
        let canonical_str = canonical_project_dir.to_string_lossy();

        // TODO(CRITICAL): LAYER VIOLATION - test code should NOT know provider directory encoding
        //
        // Current issue:
        // - Hardcoded if/else branching based on provider implementation details
        // - Duplicated logic in world.rs::get_session_file_path()
        // - Testing layer depends on Claude's "-" encoding vs Gemini's hash subdirs
        //
        // Required fix:
        // - Add `encode_project_path(project_root: &Path) -> PathBuf` to LogDiscovery trait
        // - Each provider implements its own encoding strategy
        // - Replace this if/else with: `provider_adapter.discovery.encode_project_path()`
        //
        // This abstraction belongs in agtrace-providers, NOT in test utilities.
        let project_log_dir = if let Some(provider_subdir) = provider_adapter
            .discovery
            .resolve_log_root(&canonical_project_dir)
        {
            // Provider uses project-specific subdirectory (e.g., Gemini uses hash)
            log_root.join(provider_subdir)
        } else {
            // Provider uses flat structure with encoded project names (e.g., Claude)
            let encoded = target_project_dir
                .replace(['/', '.'], "-")
                .trim_start_matches('-')
                .to_string();
            let encoded_dir = format!("-{}", encoded);
            log_root.join(encoded_dir)
        };

        fs::create_dir_all(&project_log_dir)?;

        let dest = project_log_dir.join(dest_name);

        // Read and modify content
        let content = fs::read_to_string(&source)?;

        // Replace cwd field with canonicalized path (Claude format)
        let mut modified_content = content.replace(
            r#""cwd":"/Users/test_user/agent-sample""#,
            &format!(r#""cwd":"{}""#, canonical_str),
        );

        // Replace projectHash field (Gemini format)
        // Calculate the correct project hash from the canonicalized path
        let project_hash = agtrace_types::project_hash_from_root(&canonical_str);
        modified_content = modified_content.replace(
            r#""projectHash": "9126eddec7f67e038794657b4d517dd9cb5226468f30b5ee7296c27d65e84fde""#,
            &format!(r#""projectHash": "{}""#, project_hash),
        );

        // Generate unique sessionId
        let new_session_id = generate_session_id(target_project_dir, dest_name);

        // Replace sessionId (Claude: 7f2abd2d-7cfc-4447-9ddd-3ca8d14e02e9)
        modified_content =
            modified_content.replace("7f2abd2d-7cfc-4447-9ddd-3ca8d14e02e9", &new_session_id);

        // Replace sessionId (Gemini: f0a689a6-b0ac-407f-afcc-4fafa9e14e8a)
        modified_content =
            modified_content.replace("f0a689a6-b0ac-407f-afcc-4fafa9e14e8a", &new_session_id);

        fs::write(dest, modified_content)?;
        Ok(())
    }
}

/// Generate a deterministic session ID based on project directory and filename.
fn generate_session_id(project_dir: &str, filename: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(project_dir.as_bytes());
    hasher.update(filename.as_bytes());
    let hash = hasher.finalize();

    format!(
        "test-session-{:016x}",
        u64::from_be_bytes([
            hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]
        ])
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_session_id_deterministic() {
        let id1 = generate_session_id("/Users/test/project-a", "session1.jsonl");
        let id2 = generate_session_id("/Users/test/project-a", "session1.jsonl");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_generate_session_id_unique() {
        let id1 = generate_session_id("/Users/test/project-a", "session1.jsonl");
        let id2 = generate_session_id("/Users/test/project-b", "session1.jsonl");
        assert_ne!(id1, id2);
    }
}
