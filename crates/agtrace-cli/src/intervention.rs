use agtrace_runtime::{Intervention, InterventionExecutor, ProcessTarget, Signal};
use anyhow::{anyhow, Result};
use std::io::{self, Write};
use std::process::Command;

pub struct CliInterventionExecutor;

impl Default for CliInterventionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CliInterventionExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl InterventionExecutor for CliInterventionExecutor {
    fn execute(&self, intervention: Intervention) -> Result<()> {
        match intervention {
            Intervention::Notify { title, message } => {
                println!("[notify] {} - {}", title, message);
                Ok(())
            }
            Intervention::KillProcess { target, signal } => {
                let pid = resolve_pid(target)?;
                if !confirm_kill(pid, signal)? {
                    return Err(anyhow!("User declined signal"));
                }
                send_signal(pid, signal)
            }
        }
    }
}

fn resolve_pid(target: ProcessTarget) -> Result<u32> {
    match target {
        ProcessTarget::Pid(pid) => Ok(pid),
        ProcessTarget::Name(name) => {
            let output = Command::new("pgrep")
                .arg("-f")
                .arg(&name)
                .output()
                .map_err(|e| anyhow!("pgrep failed: {}", e))?;

            if !output.status.success() {
                return Err(anyhow!(
                    "pgrep did not find process for name '{}': {}",
                    name,
                    String::from_utf8_lossy(&output.stderr)
                ));
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let pid = stdout
                .lines()
                .next()
                .ok_or_else(|| anyhow!("pgrep returned no PIDs for '{}'", name))?
                .trim()
                .parse::<u32>()
                .map_err(|e| anyhow!("Failed to parse PID: {}", e))?;

            Ok(pid)
        }
        ProcessTarget::LogFileWriter { .. } => Err(anyhow!("LogFileWriter target unsupported")),
    }
}

fn confirm_kill(pid: u32, signal: Signal) -> Result<bool> {
    print!("Send {:?} to PID {}? [y/N]: ", signal, pid);
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let normalized = input.trim().to_lowercase();
    Ok(normalized == "y" || normalized == "yes")
}

fn send_signal(pid: u32, signal: Signal) -> Result<()> {
    let sig = match signal {
        Signal::Terminate => libc::SIGTERM,
        Signal::Kill => libc::SIGKILL,
        Signal::Interrupt => libc::SIGINT,
    };

    let res = unsafe { libc::kill(pid as i32, sig) };
    if res == 0 {
        Ok(())
    } else {
        Err(anyhow!("Failed to send signal {} to {}", sig, pid))
    }
}
