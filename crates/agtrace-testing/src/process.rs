//! Background process management for long-running commands.
//!
//! Provides utilities for:
//! - Starting watch commands in the background
//! - Monitoring process output
//! - Gracefully terminating processes

use std::process::{Child, Command};
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
}

impl Drop for BackgroundProcess {
    fn drop(&mut self) {
        // Ensure process is killed when dropped
        let _ = self.child.kill();
    }
}
