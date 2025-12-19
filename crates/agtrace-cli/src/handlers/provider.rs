use crate::presentation::renderers::models::{ProviderConfigSummary, ProviderSetResult};
use crate::presentation::renderers::TraceView;
use anyhow::Result;
use std::path::PathBuf;

pub fn list(config_path: &PathBuf, view: &dyn TraceView) -> Result<()> {
    let config = crate::config::Config::load_from(config_path)?;

    let providers = config
        .providers
        .iter()
        .map(|(name, provider_config)| ProviderConfigSummary {
            name: name.clone(),
            enabled: provider_config.enabled,
            log_root: provider_config.log_root.clone(),
        })
        .collect::<Vec<_>>();

    view.render_provider_list(&providers)?;

    Ok(())
}

pub fn detect(config_path: &PathBuf, view: &dyn TraceView) -> Result<()> {
    let config = crate::config::Config::detect_providers()?;
    config.save_to(config_path)?;

    let providers = config
        .providers
        .iter()
        .map(|(name, provider_config)| ProviderConfigSummary {
            name: name.clone(),
            enabled: provider_config.enabled,
            log_root: provider_config.log_root.clone(),
        })
        .collect::<Vec<_>>();

    view.render_provider_detected(&providers)?;

    Ok(())
}

pub fn set(
    provider: String,
    log_root: PathBuf,
    enable: bool,
    disable: bool,
    config_path: &PathBuf,
    view: &dyn TraceView,
) -> Result<()> {
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
            context_window_override: None,
        },
    );

    config.save_to(config_path)?;

    view.render_provider_set(&ProviderSetResult {
        provider,
        enabled,
        log_root,
    })?;

    Ok(())
}
