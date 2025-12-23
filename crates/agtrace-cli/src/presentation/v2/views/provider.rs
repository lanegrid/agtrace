use std::fmt;

use crate::presentation::v2::view_models::{
    ProviderDetectedViewModel, ProviderListViewModel, ProviderSetViewModel,
};

// --------------------------------------------------------
// Provider List View
// --------------------------------------------------------

pub struct ProviderListView<'a> {
    data: &'a ProviderListViewModel,
}

impl<'a> ProviderListView<'a> {
    pub fn new(data: &'a ProviderListViewModel) -> Self {
        Self { data }
    }
}

impl<'a> fmt::Display for ProviderListView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

// --------------------------------------------------------
// Provider Detected View
// --------------------------------------------------------

pub struct ProviderDetectedView<'a> {
    data: &'a ProviderDetectedViewModel,
}

impl<'a> ProviderDetectedView<'a> {
    pub fn new(data: &'a ProviderDetectedViewModel) -> Self {
        Self { data }
    }
}

impl<'a> fmt::Display for ProviderDetectedView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

// --------------------------------------------------------
// Provider Set View
// --------------------------------------------------------

pub struct ProviderSetView<'a> {
    data: &'a ProviderSetViewModel,
}

impl<'a> ProviderSetView<'a> {
    pub fn new(data: &'a ProviderSetViewModel) -> Self {
        Self { data }
    }
}

impl<'a> fmt::Display for ProviderSetView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Set provider '{}': enabled={}, log_root={}",
            self.data.provider,
            self.data.enabled,
            self.data.log_root.display()
        )
    }
}
