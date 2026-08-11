use std::fmt;

use crate::presentation::view_models::{
    ViewMode, WorktreeListViewModel, WorktreeSessionsViewModel,
};

// --------------------------------------------------------
// Worktree List View
// --------------------------------------------------------

pub struct WorktreeListView<'a> {
    data: &'a WorktreeListViewModel,
    mode: ViewMode,
}

impl<'a> WorktreeListView<'a> {
    pub fn new(data: &'a WorktreeListViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for wt in &self.data.worktrees {
            writeln!(f, "{}", wt.name)?;
        }
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Repository: {} ({} worktrees)",
            &self.data.repository_hash[..8.min(self.data.repository_hash.len())],
            self.data.worktrees.len()
        )?;
        writeln!(f)?;

        for wt in &self.data.worktrees {
            let last = wt.last_active.as_deref().unwrap_or("-");
            let path_display = shorten_path(&wt.path, 50);
            writeln!(
                f,
                "  {:<16} {:>4} sessions  {}",
                wt.name, wt.session_count, last
            )?;
            writeln!(f, "    {}", path_display)?;
        }
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Repository: {} ({} worktrees)\n",
            &self.data.repository_hash[..8.min(self.data.repository_hash.len())],
            self.data.worktrees.len()
        )?;

        if self.data.worktrees.is_empty() {
            writeln!(f, "No worktrees found with indexed sessions.")?;
            return Ok(());
        }

        writeln!(
            f,
            "  {:<16} {:<40} {:>8}  {}",
            "WORKTREE", "PATH", "SESSIONS", "LAST ACTIVE"
        )?;

        for wt in &self.data.worktrees {
            let last = wt.last_active.as_deref().unwrap_or("-");
            let path_display = shorten_path(&wt.path, 40);
            writeln!(
                f,
                "  {:<16} {:<40} {:>8}  {}",
                wt.name, path_display, wt.session_count, last
            )?;
        }
        Ok(())
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.render_standard(f)
    }
}

impl<'a> fmt::Display for WorktreeListView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}

// --------------------------------------------------------
// Worktree Sessions View
// --------------------------------------------------------

pub struct WorktreeSessionsView<'a> {
    data: &'a WorktreeSessionsViewModel,
    mode: ViewMode,
}

impl<'a> WorktreeSessionsView<'a> {
    pub fn new(data: &'a WorktreeSessionsViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for group in &self.data.groups {
            for session in &group.sessions {
                writeln!(f, "{}", session.id_short)?;
            }
        }
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Repository: {} ({} sessions across {} worktrees)\n",
            &self.data.repository_hash[..8.min(self.data.repository_hash.len())],
            self.data.total_sessions,
            self.data.groups.len()
        )?;

        for group in &self.data.groups {
            writeln!(f, "{}/  ({} sessions)", group.name, group.sessions.len())?;
            for session in &group.sessions {
                writeln!(
                    f,
                    "  {}  {:>12}  \"{}\"",
                    session.id_short, session.time, session.snippet
                )?;
            }
            writeln!(f)?;
        }
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Repository: {} ({} sessions across {} worktrees)\n",
            &self.data.repository_hash[..8.min(self.data.repository_hash.len())],
            self.data.total_sessions,
            self.data.groups.len()
        )?;

        if self.data.groups.is_empty() {
            writeln!(f, "No sessions found in any worktree.")?;
            return Ok(());
        }

        for group in &self.data.groups {
            writeln!(f, "{}/  ({} sessions)", group.name, group.sessions.len())?;
            writeln!(f, "  {}", shorten_path(&group.path, 60))?;

            for session in &group.sessions {
                writeln!(
                    f,
                    "    {}  {:>12}  \"{}\"",
                    session.id_short, session.time, session.snippet
                )?;
            }
            writeln!(f)?;
        }
        Ok(())
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.render_standard(f)
    }
}

impl<'a> fmt::Display for WorktreeSessionsView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}

// --------------------------------------------------------
// Helper Functions
// --------------------------------------------------------

fn shorten_path(path: &str, max_len: usize) -> String {
    // Always try to shorten home directory first for consistency
    let path = if let Some(home) = home_dir() {
        if let Some(rest) = path.strip_prefix(&home) {
            format!("~{}", rest)
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    };

    if path.len() <= max_len {
        return path;
    }

    // Truncate from the beginning
    let suffix = &path[path.len().saturating_sub(max_len - 3)..];
    format!("...{}", suffix)
}

fn home_dir() -> Option<String> {
    std::env::var("HOME").ok()
}
