//! Background process management for long-running commands.
//!
//! Provides utilities for:
//! - Starting watch commands in the background
//! - Monitoring process output
//! - Gracefully terminating processes

use std::process::{Child, ChildStdout, Command, Stdio};
use std::time::Duration;

/// A background process handle.
pub struct BackgroundProcess {
    child: Child,
}

impl BackgroundProcess {
    /// Spawn a new background process.
    pub fn spawn(mut command: Command) -> std::io::Result<Self> {
        let child = command.spawn()?;
        Ok(Self { child })
    }

    /// Spawn a new background process with piped stdout and stderr.
    ///
    /// This allows reading the process output while it's running.
    ///
    /// # Example
    /// ```no_run
    /// # use std::process::Command;
    /// # use agtrace_testing::process::BackgroundProcess;
    /// # use std::io::{BufRead, BufReader};
    /// let mut cmd = Command::new("agtrace");
    /// cmd.args(&["watch"]);
    ///
    /// let mut proc = BackgroundProcess::spawn_piped(cmd).unwrap();
    ///
    /// // Read output
    /// if let Some(stdout) = proc.stdout() {
    ///     let reader = BufReader::new(stdout);
    ///     for line in reader.lines() {
    ///         println!("Output: {:?}", line);
    ///     }
    /// }
    /// ```
    pub fn spawn_piped(mut command: Command) -> std::io::Result<Self> {
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let child = command.spawn()?;
        Ok(Self { child })
    }

    /// Wait for the process to exit with a timeout.
    pub fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>> {
        // Simple polling implementation
        let start = std::time::Instant::now();
        loop {
            match self.child.try_wait()? {
                Some(status) => return Ok(Some(status)),
                None => {
                    if start.elapsed() > timeout {
                        return Ok(None);
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }

    /// Kill the process.
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }

    /// Get the process ID.
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// Get mutable access to the process's stdout.
    ///
    /// Returns `None` if stdout was not captured (process must be spawned with `spawn_piped`).
    ///
    /// # Example
    /// ```no_run
    /// # use std::process::Command;
    /// # use agtrace_testing::process::BackgroundProcess;
    /// # use std::io::{BufRead, BufReader};
    /// let mut cmd = Command::new("agtrace");
    /// let mut proc = BackgroundProcess::spawn_piped(cmd).unwrap();
    ///
    /// if let Some(stdout) = proc.stdout() {
    ///     let reader = BufReader::new(stdout);
    ///     for line in reader.lines().take(5) {
    ///         println!("{:?}", line);
    ///     }
    /// }
    /// ```
    pub fn stdout(&mut self) -> Option<&mut ChildStdout> {
        self.child.stdout.as_mut()
    }

    /// Get mutable access to the process's stderr.
    ///
    /// Returns `None` if stderr was not captured (process must be spawned with `spawn_piped`).
    pub fn stderr(&mut self) -> Option<&mut std::process::ChildStderr> {
        self.child.stderr.as_mut()
    }
}

impl Drop for BackgroundProcess {
    fn drop(&mut self) {
        // Ensure process is killed when dropped
        let _ = self.child.kill();
    }
}
