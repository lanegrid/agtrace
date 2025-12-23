use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

use super::{CreateView, ViewMode};

#[derive(Debug, Serialize)]
pub struct ProviderListViewModel {
    pub providers: Vec<ProviderEntry>,
}

#[derive(Debug, Serialize)]
pub struct ProviderEntry {
    pub name: String,
    pub enabled: bool,
    pub log_root: PathBuf,
}

impl CreateView for ProviderListViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(ProviderListView { data: self })
    }
}

struct ProviderListView<'a> {
    data: &'a ProviderListViewModel,
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

impl fmt::Display for ProviderListViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", ProviderListView { data: self })
    }
}

#[derive(Debug, Serialize)]
pub struct ProviderDetectedViewModel {
    pub providers: Vec<ProviderEntry>,
}

impl CreateView for ProviderDetectedViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(ProviderDetectedView { data: self })
    }
}

struct ProviderDetectedView<'a> {
    data: &'a ProviderDetectedViewModel,
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

impl fmt::Display for ProviderDetectedViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", ProviderDetectedView { data: self })
    }
}

#[derive(Debug, Serialize)]
pub struct ProviderSetViewModel {
    pub provider: String,
    pub enabled: bool,
    pub log_root: PathBuf,
}

impl CreateView for ProviderSetViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(ProviderSetView { data: self })
    }
}

struct ProviderSetView<'a> {
    data: &'a ProviderSetViewModel,
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

impl fmt::Display for ProviderSetViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", ProviderSetView { data: self })
    }
}
