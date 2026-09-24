//! Process liveness for the Claude session registry.

/// True if a process with this pid exists (unix: `kill(pid, 0)` succeeds or fails
/// with `EPERM`, i.e. the process exists but belongs to someone else).
/// Non-unix: registry presence only (always true).
#[cfg(unix)]
pub(crate) fn pid_alive(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    if pid <= 0 {
        return false;
    }
    // SAFETY: signal 0 performs error checking only; no signal is delivered.
    let rc = unsafe { libc::kill(pid, 0) };
    rc == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(not(unix))]
pub(crate) fn pid_alive(_pid: u32) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_process_is_alive() {
        assert!(pid_alive(std::process::id()));
    }

    #[cfg(unix)]
    #[test]
    fn invalid_pids_are_dead() {
        assert!(!pid_alive(0));
        assert!(!pid_alive(u32::MAX));
    }
}
