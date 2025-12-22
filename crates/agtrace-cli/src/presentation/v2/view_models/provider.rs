use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

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

impl fmt::Display for ProviderListViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.providers.is_empty() {
            writeln!(f, "No providers configured.")?;
            return Ok(());
        }

        writeln!(f, "{:<15} {:<10} LOG_ROOT", "PROVIDER", "ENABLED")?;
        writeln!(f, "{}", "-".repeat(80))?;

        for provider in &self.providers {
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

#[derive(Debug, Serialize)]
pub struct ProviderDetectedViewModel {
    pub providers: Vec<ProviderEntry>,
}

impl fmt::Display for ProviderDetectedViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.providers.is_empty() {
            writeln!(f, "No providers detected.")?;
            return Ok(());
        }

        writeln!(f, "Detected {} provider(s):", self.providers.len())?;
        for provider in &self.providers {
            writeln!(f, "  {} -> {}", provider.name, provider.log_root.display())?;
        }

        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct ProviderSetViewModel {
    pub provider: String,
    pub enabled: bool,
    pub log_root: PathBuf,
}

impl fmt::Display for ProviderSetViewModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Set provider '{}': enabled={}, log_root={}",
            self.provider,
            self.enabled,
            self.log_root.display()
        )
    }
}
