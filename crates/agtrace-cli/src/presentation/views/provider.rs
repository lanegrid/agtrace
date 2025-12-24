use std::fmt;

use crate::presentation::view_models::{
    ProviderDetectedViewModel, ProviderListViewModel, ProviderSetViewModel, ViewMode,
};

// --------------------------------------------------------
// Provider List View
// --------------------------------------------------------

pub struct ProviderListView<'a> {
    data: &'a ProviderListViewModel,
    mode: ViewMode,
}

impl<'a> ProviderListView<'a> {
    pub fn new(data: &'a ProviderListViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for provider in &self.data.providers {
            writeln!(f, "{}", provider.name)?;
        }
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.data.providers.is_empty() {
            writeln!(f, "No providers")?;
            return Ok(());
        }

        for provider in &self.data.providers {
            let status = if provider.enabled { "✓" } else { "✗" };
            writeln!(f, "{} {}", status, provider.name)?;
        }
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.data.providers.is_empty() {
            writeln!(f, "No providers configured.")?;
            return Ok(());
        }

        writeln!(f, "{:<15} {:<10} LOG_ROOT", "PROVIDER", "ENABLED")?;
        writeln!(f, "{}", "-".repeat(80))?;

        for provider in &self.data.providers {
            writeln!(
                f,
                "{:<15} {:<10} {}",
                provider.name,
                if provider.enabled { "yes" } else { "no" },
                provider.log_root.display()
            )?;
        }

        Ok(())
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.data.providers.is_empty() {
            writeln!(f, "No providers configured.")?;
            return Ok(());
        }

        writeln!(f, "{:<15} {:<10} LOG_ROOT", "PROVIDER", "ENABLED")?;
        writeln!(f, "{}", "-".repeat(80))?;

        for provider in &self.data.providers {
            writeln!(
                f,
                "{:<15} {:<10} {}",
                provider.name,
                if provider.enabled { "yes" } else { "no" },
                provider.log_root.display()
            )?;
        }

        Ok(())
    }
}

impl<'a> fmt::Display for ProviderListView<'a> {
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
// Provider Detected View
// --------------------------------------------------------

pub struct ProviderDetectedView<'a> {
    data: &'a ProviderDetectedViewModel,
    mode: ViewMode,
}

impl<'a> ProviderDetectedView<'a> {
    pub fn new(data: &'a ProviderDetectedViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.data.providers.len())?;
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.data.providers.is_empty() {
            writeln!(f, "No providers detected")?;
            return Ok(());
        }

        for provider in &self.data.providers {
            writeln!(f, "{}", provider.name)?;
        }
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.data.providers.is_empty() {
            writeln!(f, "No providers detected.")?;
            return Ok(());
        }

        writeln!(f, "Detected {} provider(s):", self.data.providers.len())?;
        for provider in &self.data.providers {
            writeln!(f, "  {} -> {}", provider.name, provider.log_root.display())?;
        }

        Ok(())
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.data.providers.is_empty() {
            writeln!(f, "No providers detected.")?;
            return Ok(());
        }

        writeln!(f, "Detected {} provider(s):", self.data.providers.len())?;
        for provider in &self.data.providers {
            writeln!(f, "  {} -> {}", provider.name, provider.log_root.display())?;
        }

        Ok(())
    }
}

impl<'a> fmt::Display for ProviderDetectedView<'a> {
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
// Provider Set View
// --------------------------------------------------------

pub struct ProviderSetView<'a> {
    data: &'a ProviderSetViewModel,
    mode: ViewMode,
}

impl<'a> ProviderSetView<'a> {
    pub fn new(data: &'a ProviderSetViewModel, mode: ViewMode) -> Self {
        Self { data, mode }
    }

    fn render_minimal(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.data.provider)?;
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let status = if self.data.enabled { "✓" } else { "✗" };
        writeln!(f, "{} {}", status, self.data.provider)?;
        Ok(())
    }

    fn render_standard(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Set provider '{}': enabled={}, log_root={}",
            self.data.provider,
            self.data.enabled,
            self.data.log_root.display()
        )
    }

    fn render_verbose(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Set provider '{}': enabled={}, log_root={}",
            self.data.provider,
            self.data.enabled,
            self.data.log_root.display()
        )
    }
}

impl<'a> fmt::Display for ProviderSetView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.mode {
            ViewMode::Minimal => self.render_minimal(f),
            ViewMode::Compact => self.render_compact(f),
            ViewMode::Standard => self.render_standard(f),
            ViewMode::Verbose => self.render_verbose(f),
        }
    }
}
