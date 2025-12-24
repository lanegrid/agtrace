use std::path::PathBuf;

use crate::presentation::view_models::{
    CommandResultViewModel, Guidance, ProviderDetectedViewModel, ProviderEntry,
    ProviderListViewModel, ProviderSetViewModel, StatusBadge,
};

pub fn present_provider_list(
    providers: Vec<(String, bool, PathBuf)>,
) -> CommandResultViewModel<ProviderListViewModel> {
    let entries: Vec<ProviderEntry> = providers
        .into_iter()
        .map(|(name, enabled, log_root)| ProviderEntry {
            name,
            enabled,
            log_root,
        })
        .collect();

    let content = ProviderListViewModel { providers: entries };

    let mut result = CommandResultViewModel::new(content);

    if result.content.providers.is_empty() {
        result = result
            .with_badge(StatusBadge::warning("No providers configured"))
            .with_suggestion(
                Guidance::new("Auto-detect providers").with_command("agtrace provider detect"),
            )
            .with_suggestion(
                Guidance::new("Or manually configure a provider")
                    .with_command("agtrace provider set <name> --log-root <PATH> --enable"),
            );
    } else {
        let enabled_count = result
            .content
            .providers
            .iter()
            .filter(|p| p.enabled)
            .count();
        let label = format!(
            "{} provider(s), {} enabled",
            result.content.providers.len(),
            enabled_count
        );
        result = result.with_badge(StatusBadge::success(label));
    }

    result
}

pub fn present_provider_detected(
    providers: Vec<(String, bool, PathBuf)>,
) -> CommandResultViewModel<ProviderDetectedViewModel> {
    let entries: Vec<ProviderEntry> = providers
        .into_iter()
        .map(|(name, enabled, log_root)| ProviderEntry {
            name,
            enabled,
            log_root,
        })
        .collect();

    let content = ProviderDetectedViewModel { providers: entries };

    let mut result = CommandResultViewModel::new(content);

    if result.content.providers.is_empty() {
        result = result
            .with_badge(StatusBadge::warning("No providers detected"))
            .with_suggestion(
                Guidance::new("Manually configure a provider")
                    .with_command("agtrace provider set <name> --log-root <PATH> --enable"),
            );
    } else {
        let label = format!(
            "{} provider(s) detected and saved",
            result.content.providers.len()
        );
        result = result
            .with_badge(StatusBadge::success(label))
            .with_suggestion(
                Guidance::new("View configured providers").with_command("agtrace provider list"),
            )
            .with_suggestion(
                Guidance::new("Start indexing sessions").with_command("agtrace index update"),
            );
    }

    result
}

pub fn present_provider_set(
    provider: String,
    enabled: bool,
    log_root: PathBuf,
) -> CommandResultViewModel<ProviderSetViewModel> {
    let content = ProviderSetViewModel {
        provider: provider.clone(),
        enabled,
        log_root,
    };

    let label = if enabled {
        format!("Provider '{}' enabled", provider)
    } else {
        format!("Provider '{}' disabled", provider)
    };

    CommandResultViewModel::new(content)
        .with_badge(StatusBadge::success(label))
        .with_suggestion(Guidance::new("View all providers").with_command("agtrace provider list"))
        .with_suggestion(
            Guidance::new("Index sessions from this provider").with_command("agtrace index update"),
        )
}
