use crate::args::{OutputFormat, ViewModeArgs};
use agtrace_runtime::{Config, ProviderConfig};
use anyhow::Result;
use std::path::PathBuf;

pub fn list_v2(
    config_path: &PathBuf,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    use crate::presentation::presenters;
    use crate::presentation::{ConsoleRenderer, Renderer};

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

    let v2_format = crate::presentation::OutputFormat::from(format);
    let resolved_view_mode = view_mode.resolve();
    let renderer = ConsoleRenderer::new(v2_format, resolved_view_mode);
    renderer.render(view_model)?;

    Ok(())
}

pub fn detect_v2(
    config_path: &PathBuf,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    use crate::presentation::presenters;
    use crate::presentation::{ConsoleRenderer, Renderer};

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

    let v2_format = crate::presentation::OutputFormat::from(format);
    let resolved_view_mode = view_mode.resolve();
    let renderer = ConsoleRenderer::new(v2_format, resolved_view_mode);
    renderer.render(view_model)?;

    Ok(())
}

pub fn set_v2(
    provider: String,
    log_root: PathBuf,
    enable: bool,
    disable: bool,
    config_path: &PathBuf,
    format: OutputFormat,
    view_mode: &ViewModeArgs,
) -> Result<()> {
    use crate::presentation::presenters;
    use crate::presentation::{ConsoleRenderer, Renderer};

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

    let v2_format = crate::presentation::OutputFormat::from(format);
    let resolved_view_mode = view_mode.resolve();
    let renderer = ConsoleRenderer::new(v2_format, resolved_view_mode);
    renderer.render(view_model)?;

    Ok(())
}
