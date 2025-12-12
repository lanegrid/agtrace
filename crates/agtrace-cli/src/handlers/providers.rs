use crate::args::ProvidersCommand;
use anyhow::Result;
use std::path::PathBuf;

pub fn handle(command: Option<ProvidersCommand>, config_path: &PathBuf) -> Result<()> {
    match command {
        None | Some(ProvidersCommand::List) => {
            let config = crate::config::Config::load_from(config_path)?;

            if config.providers.is_empty() {
                println!("No providers configured. Run 'agtrace providers detect' to auto-detect.");
                return Ok(());
            }

            println!("{:<15} {:<10} LOG_ROOT", "PROVIDER", "ENABLED");
            println!("{}", "-".repeat(80));

            for (name, provider_config) in &config.providers {
                println!(
                    "{:<15} {:<10} {}",
                    name,
                    if provider_config.enabled { "yes" } else { "no" },
                    provider_config.log_root.display()
                );
            }
        }

        Some(ProvidersCommand::Detect) => {
            let config = crate::config::Config::detect_providers()?;
            config.save_to(config_path)?;

            println!("Detected {} provider(s):", config.providers.len());
            for (name, provider_config) in &config.providers {
                println!("  {} -> {}", name, provider_config.log_root.display());
            }
        }

        Some(ProvidersCommand::Set {
            provider,
            log_root,
            enable,
            disable,
        }) => {
            if enable && disable {
                anyhow::bail!("Cannot specify both --enable and --disable");
            }

            let mut config = crate::config::Config::load_from(config_path)?;

            let enabled = if enable { true } else { !disable };

            config.set_provider(
                provider.clone(),
                crate::config::ProviderConfig {
                    enabled,
                    log_root: log_root.clone(),
                },
            );

            config.save_to(config_path)?;

            println!(
                "Set provider '{}': enabled={}, log_root={}",
                provider,
                enabled,
                log_root.display()
            );
        }
    }

    Ok(())
}
