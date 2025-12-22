use serde::Serialize;
use std::path::PathBuf;

use crate::presentation::v2::renderers::ConsolePresentable;

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

impl ConsolePresentable for ProviderListViewModel {
    fn render_console(&self) {
        if self.providers.is_empty() {
            println!("No providers configured.");
            return;
        }

        println!("{:<15} {:<10} LOG_ROOT", "PROVIDER", "ENABLED");
        println!("{}", "-".repeat(80));

        for provider in &self.providers {
            println!(
                "{:<15} {:<10} {}",
                provider.name,
                if provider.enabled { "yes" } else { "no" },
                provider.log_root.display()
            );
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProviderDetectedViewModel {
    pub providers: Vec<ProviderEntry>,
}

impl ConsolePresentable for ProviderDetectedViewModel {
    fn render_console(&self) {
        if self.providers.is_empty() {
            println!("No providers detected.");
            return;
        }

        println!("Detected {} provider(s):", self.providers.len());
        for provider in &self.providers {
            println!("  {} -> {}", provider.name, provider.log_root.display());
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProviderSetViewModel {
    pub provider: String,
    pub enabled: bool,
    pub log_root: PathBuf,
}

impl ConsolePresentable for ProviderSetViewModel {
    fn render_console(&self) {
        println!(
            "Set provider '{}': enabled={}, log_root={}",
            self.provider,
            self.enabled,
            self.log_root.display()
        );
    }
}
