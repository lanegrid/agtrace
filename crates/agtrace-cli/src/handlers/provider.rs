use crate::args::{OutputFormat, ViewModeArgs};
use crate::handlers::HandlerContext;
use crate::presentation::presenters;
use agtrace_sdk::types::{Config, ProviderConfig};
use anyhow::Result;
use std::path::PathBuf;

pub fn list(config_path: &PathBuf, format: OutputFormat, view_mode: &ViewModeArgs) -> Result<()> {
    let ctx = HandlerContext::new(format, view_mode);
    let config = Config::load_from(config_path)?;

    let providers: Vec<(String, bool, PathBuf)> = config
        .providers
        .iter()
        .map(|(name, provider_config)| {
            (
                name.clone(),
                provider_config.enabled,
                provider_config.log_root.clone(),
            )
        })
        .collect();

    let view_model = presenters::present_provider_list(providers);
    ctx.render(view_model)
}

pub fn detect(config_path: &PathBuf, format: OutputFormat, view_mode: &ViewModeArgs) -> Result<()> {
    let ctx = HandlerContext::new(format, view_mode);
    let config = Config::detect_providers()?;
    config.save_to(config_path)?;

    let providers: Vec<(String, bool, PathBuf)> = config
        .providers
        .iter()
        .map(|(name, provider_config)| {
            (
                name.clone(),
                provider_config.enabled,
                provider_config.log_root.clone(),
            )
        })
        .collect();

    let view_model = presenters::present_provider_detected(providers);
    ctx.render(view_model)
}

pub fn set(
    provider: String,
    log_root: PathBuf,
    enable: bool,
    disable: bool,
    config_path: &PathBuf,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    let ctx = HandlerContext::new(format, view_mode);

    if enable && disable {
        anyhow::bail!("Cannot specify both --enable and --disable");
    }

    let mut config = Config::load_from(config_path)?;
    let enabled = if enable { true } else { !disable };

    config.set_provider(
        provider.clone(),
        ProviderConfig {
            enabled,
            log_root: log_root.clone(),
            context_window_override: None,
        },
    );

    config.save_to(config_path)?;

    let view_model = presenters::present_provider_set(provider, enabled, log_root);
    ctx.render(view_model)
}
