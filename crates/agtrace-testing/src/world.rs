//! TestWorld pattern for declarative integration test setup.
//!
//! Provides a fluent interface for:
//! - Creating isolated test environments
//! - Managing working directories
//! - Setting up sample data
//! - Executing CLI commands with proper context

use anyhow::Result;
use assert_cmd::Command;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::fixtures::SampleFiles;

/// Declarative test environment builder.
///
/// # Example
/// ```no_run
/// use agtrace_testing::TestWorld;
///
/// let world = TestWorld::new()
///     .with_project("project-a")
///     .enter_dir("project-a");
///
/// let result = world.run(&["session", "list"]).unwrap();
/// assert!(result.success());
/// ```
pub struct TestWorld {
    temp_dir: TempDir,
    cwd: PathBuf,
    data_dir: PathBuf,
    log_root: PathBuf,
    env_vars: HashMap<String, String>,
    samples: SampleFiles,
}

impl Default for TestWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl TestWorld {
    /// Create a new isolated test environment.
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let base_path = temp_dir.path().to_path_buf();
        let data_dir = base_path.join(".agtrace");
        let log_root = base_path.join(".claude");

        std::fs::create_dir_all(&data_dir).expect("Failed to create data dir");
        std::fs::create_dir_all(&log_root).expect("Failed to create log dir");

        Self {
            cwd: base_path.clone(),
            temp_dir,
            data_dir,
            log_root,
            env_vars: HashMap::new(),
            samples: SampleFiles::new(),
        }
    }

    /// Get the data directory path (.agtrace).
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Get the log root directory path (.claude).
    pub fn log_root(&self) -> &Path {
        &self.log_root
    }

    /// Get the current working directory.
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// Get the temp directory root.
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Change the current working directory (relative to temp root).
    ///
    /// This is crucial for testing CWD-dependent logic.
    /// This method consumes `self` for use in builder pattern chains.
    ///
    /// For changing directory multiple times in a test, use `set_cwd()` instead.
    pub fn enter_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        let new_cwd = if path.as_ref().is_absolute() {
            path.as_ref().to_path_buf()
        } else {
            self.temp_dir.path().join(path)
        };

        // Create the directory if it doesn't exist
        std::fs::create_dir_all(&new_cwd).expect("Failed to create directory");
        self.cwd = new_cwd;
        self
    }

    /// Set the current working directory without consuming self.
    ///
    /// This is useful when you need to change directories multiple times
    /// during a test.
    ///
    /// # Example
    /// ```no_run
    /// # use agtrace_testing::TestWorld;
    /// let mut world = TestWorld::new()
    ///     .with_project("project-a")
    ///     .with_project("project-b");
    ///
    /// // Move to project-a
    /// world.set_cwd("project-a");
    /// let result = world.run(&["session", "list"]).unwrap();
    ///
    /// // Move to project-b
    /// world.set_cwd("project-b");
    /// let result = world.run(&["session", "list"]).unwrap();
    /// ```
    pub fn set_cwd<P: AsRef<Path>>(&mut self, path: P) {
        let new_cwd = if path.as_ref().is_absolute() {
            path.as_ref().to_path_buf()
        } else {
            self.temp_dir.path().join(path)
        };

        // Create the directory if it doesn't exist
        std::fs::create_dir_all(&new_cwd).expect("Failed to create directory");
        self.cwd = new_cwd;
    }

    /// Set an environment variable for CLI execution.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Create a project directory structure.
    pub fn with_project(self, project_name: &str) -> Self {
        let project_dir = self.temp_dir.path().join(project_name);
        std::fs::create_dir_all(&project_dir).expect("Failed to create project dir");
        self
    }

    /// Configure a CLI command with this test environment's settings.
    ///
    /// The caller must provide the base command (e.g., from `cargo_bin_cmd!("agtrace")`).
    /// This method configures it with the appropriate data-dir, cwd, and env vars.
    pub fn configure_command<'a>(&self, cmd: &'a mut Command) -> &'a mut Command {
        cmd.arg("--data-dir")
            .arg(self.data_dir())
            .arg("--format")
            .arg("plain");

        // Set CWD for the command
        cmd.current_dir(&self.cwd);

        // Apply environment variables
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        cmd
    }

    /// Create a CLI command configured for this test environment.
    ///
    /// Note: This requires the binary to be built and available in the cargo target directory.
    /// For integration tests, prefer using `configure_command` with `cargo_bin_cmd!("agtrace")`.
    #[doc(hidden)]
    pub fn command_from_path(&self, bin_path: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut cmd = Command::new(bin_path);
        self.configure_command(&mut cmd);
        cmd
    }

    /// Copy a sample file to the log root.
    pub fn copy_sample(&self, sample_name: &str, dest_name: &str) -> Result<()> {
        let dest = self.log_root.join(dest_name);
        self.samples.copy_to(sample_name, &dest)
    }

    /// Copy a sample file to a Claude-encoded project directory.
    pub fn copy_sample_to_project(
        &self,
        sample_name: &str,
        dest_name: &str,
        project_dir: &str,
    ) -> Result<()> {
        self.samples
            .copy_to_project(sample_name, dest_name, project_dir, &self.log_root)
    }

    /// Copy a sample file to a project with cwd and sessionId replacement.
    ///
    /// This is the recommended method for creating isolated test sessions.
    pub fn copy_sample_to_project_with_cwd(
        &self,
        sample_name: &str,
        dest_name: &str,
        target_project_dir: &str,
    ) -> Result<()> {
        self.samples.copy_to_project_with_cwd(
            sample_name,
            dest_name,
            target_project_dir,
            &self.log_root,
        )
    }

    /// Execute a command using the project's binary and return the result.
    ///
    /// This is a convenience method that creates a command, configures it
    /// with the test environment settings, and executes it.
    ///
    /// # Example
    /// ```no_run
    /// # use agtrace_testing::TestWorld;
    /// let world = TestWorld::new();
    /// let result = world.run(&["session", "list"]).unwrap();
    /// assert!(result.success());
    /// ```
    ///
    /// # Note
    /// This method uses `Command::cargo_bin()` which requires the binary to be
    /// built and the `CARGO_BIN_EXE_` environment variable to be set (which
    /// cargo test does automatically).
    #[allow(deprecated)]
    pub fn run(&self, args: &[&str]) -> Result<CliResult> {
        // Find the binary using cargo_bin
        let mut cmd = Command::cargo_bin("agtrace")
            .map_err(|e| anyhow::anyhow!("Failed to find agtrace binary: {}", e))?;

        // Configure with test environment settings
        self.configure_command(&mut cmd);

        // Add arguments
        cmd.args(args);

        // Execute
        let output = cmd.output()?;

        Ok(CliResult {
            status: output.status,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    /// Execute a command in a specific directory temporarily.
    ///
    /// This helper temporarily changes the working directory, runs the command,
    /// and restores the original directory. This is useful for testing commands
    /// that depend on the current working directory without permanently changing
    /// the `TestWorld` state.
    ///
    /// # Example
    /// ```no_run
    /// # use agtrace_testing::TestWorld;
    /// let mut world = TestWorld::new()
    ///     .with_project("project-a")
    ///     .with_project("project-b");
    ///
    /// // Run in project-a
    /// let result_a = world.run_in_dir(&["session", "list"], "project-a").unwrap();
    ///
    /// // Run in project-b (without manually changing cwd)
    /// let result_b = world.run_in_dir(&["session", "list"], "project-b").unwrap();
    ///
    /// // Original cwd is preserved
    /// ```
    pub fn run_in_dir<P: AsRef<Path>>(&mut self, args: &[&str], dir: P) -> Result<CliResult> {
        // Save original cwd
        let original_cwd = self.cwd.clone();

        // Temporarily change directory
        self.set_cwd(dir);

        // Run the command
        let result = self.run(args);

        // Restore original directory
        self.cwd = original_cwd;

        result
    }
}

/// Result of a CLI command execution.
#[derive(Debug)]
pub struct CliResult {
    pub status: std::process::ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl CliResult {
    /// Check if the command succeeded.
    pub fn success(&self) -> bool {
        self.status.success()
    }

    /// Parse stdout as JSON.
    pub fn json(&self) -> Result<serde_json::Value> {
        Ok(serde_json::from_str(&self.stdout)?)
    }

    /// Get stdout as a string.
    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    /// Get stderr as a string.
    pub fn stderr(&self) -> &str {
        &self.stderr
    }
}
