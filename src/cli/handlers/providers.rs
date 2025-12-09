use crate::cli::args::ProvidersCommand;
use anyhow::Result;

pub fn handle(command: Option<ProvidersCommand>) -> Result<()> {
    match command {
        None | Some(ProvidersCommand::List) => {
            let config = crate::config::Config::load()?;

            if config.providers.is_empty() {
                println!("No providers configured. Run 'agtrace providers detect' to auto-detect.");
                return Ok(());
            }

            println!("{:<15} {:<10} {}", "PROVIDER", "ENABLED", "LOG_ROOT");
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
            config.save()?;

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

            let mut config = crate::config::Config::load()?;

            let enabled = if enable {
                true
            } else if disable {
                false
            } else {
                true
            };

            config.set_provider(
                provider.clone(),
                crate::config::ProviderConfig {
                    enabled,
                    log_root: log_root.clone(),
                },
            );

            config.save()?;

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
