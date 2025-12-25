//! Common test utilities shared across integration tests.
//!
//! Note: Clippy cannot track usage across integration test files,
//! hence the `allow(dead_code)` annotation. This is a standard pattern
//! for Rust integration test fixtures.
//!
//! See: docs/testing_best_practices.md
#![cfg(test)]
#![allow(dead_code)]

use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

pub struct TestFixture {
    _temp_dir: TempDir,
    data_dir: PathBuf,
    log_root: PathBuf,
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}

impl TestFixture {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let data_dir = temp_dir.path().join(".agtrace");
        let log_root = temp_dir.path().join(".claude");

        fs::create_dir_all(&data_dir).expect("Failed to create data dir");
        fs::create_dir_all(&log_root).expect("Failed to create log dir");

        Self {
            _temp_dir: temp_dir,
            data_dir,
            log_root,
        }
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub fn log_root(&self) -> &PathBuf {
        &self.log_root
    }

    pub fn copy_sample_file(&self, sample_name: &str, dest_name: &str) -> anyhow::Result<()> {
        let samples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("agtrace-providers/tests/samples");

        let source = samples_dir.join(sample_name);
        let dest = self.log_root.join(dest_name);

        fs::copy(source, dest)?;
        Ok(())
    }

    /// Copy sample file to a Claude-encoded project directory
    /// Claude encodes project paths like: /Users/foo/bar -> -Users-foo-bar
    pub fn copy_sample_file_to_project(
        &self,
        sample_name: &str,
        dest_name: &str,
        project_dir: &str,
    ) -> anyhow::Result<()> {
        let samples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("agtrace-providers/tests/samples");

        let source = samples_dir.join(sample_name);

        // Encode project directory (Claude format)
        let encoded = project_dir
            .replace(['/', '.'], "-")
            .trim_start_matches('-')
            .to_string();
        let encoded_dir = format!("-{}", encoded);

        let project_log_dir = self.log_root.join(encoded_dir);
        fs::create_dir_all(&project_log_dir)?;

        let dest = project_log_dir.join(dest_name);
        fs::copy(source, dest)?;
        Ok(())
    }

    /// Copy sample file to a Claude-encoded project directory with cwd replacement
    /// This replaces the embedded cwd and sessionId fields to create independent test sessions
    pub fn copy_sample_file_to_project_with_cwd(
        &self,
        sample_name: &str,
        dest_name: &str,
        target_project_dir: &str,
    ) -> anyhow::Result<()> {
        let samples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("agtrace-providers/tests/samples");

        let source = samples_dir.join(sample_name);

        // Encode project directory (Claude format)
        let encoded = target_project_dir
            .replace(['/', '.'], "-")
            .trim_start_matches('-')
            .to_string();
        let encoded_dir = format!("-{}", encoded);

        let project_log_dir = self.log_root.join(encoded_dir);
        fs::create_dir_all(&project_log_dir)?;

        let dest = project_log_dir.join(dest_name);

        // Read the source file and replace cwd fields
        let content = fs::read_to_string(&source)?;

        // Replace cwd field
        let mut modified_content = content.replace(
            r#""cwd":"/Users/test_user/agent-sample""#,
            &format!(r#""cwd":"{}""#, target_project_dir)
        );

        // Generate a unique sessionId based on the target project dir AND dest file name
        // This ensures each test file gets its own sessionId
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        target_project_dir.hash(&mut hasher);
        dest_name.hash(&mut hasher);
        let hash = hasher.finish();
        let new_session_id = format!("test-session-{:016x}", hash);

        // Replace sessionId (original: 7f2abd2d-7cfc-4447-9ddd-3ca8d14e02e9)
        modified_content = modified_content.replace(
            "7f2abd2d-7cfc-4447-9ddd-3ca8d14e02e9",
            &new_session_id
        );

        fs::write(dest, modified_content)?;
        Ok(())
    }

    pub fn command(&self) -> Command {
        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("agtrace");
        cmd.arg("--data-dir")
            .arg(self.data_dir())
            .arg("--format")
            .arg("plain");
        cmd
    }

    pub fn setup_provider(&self, provider_name: &str) -> anyhow::Result<()> {
        let mut cmd = self.command();
        let output = cmd
            .arg("provider")
            .arg("set")
            .arg(provider_name)
            .arg("--log-root")
            .arg(self.log_root())
            .arg("--enable")
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "provider set failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    pub fn index_update(&self) -> anyhow::Result<()> {
        let mut cmd = self.command();
        let output = cmd
            .arg("index")
            .arg("update")
            .arg("--all-projects")
            .arg("--verbose")
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "index update failed: {}\nstdout: {}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            );
        }
        Ok(())
    }
}
