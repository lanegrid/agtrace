use crate::presentation::presenters;
use crate::presentation::renderers::TraceView;
use agtrace_providers::{create_provider, detect_provider_from_path, ImportContext, LogProvider};
use anyhow::Result;
use std::path::Path;

pub fn handle(
    file_path: String,
    provider_override: Option<String>,
    view: &dyn TraceView,
) -> Result<()> {
    let path = Path::new(&file_path);

    if !path.exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    // Auto-detect or use specified provider
    let (provider, provider_name): (Box<dyn LogProvider>, String) =
        if let Some(name) = provider_override {
            let provider = create_provider(&name)?;
            (provider, name)
        } else {
            let provider = detect_provider_from_path(&file_path)?;
            let name = format!("{} (auto-detected)", provider.name());
            (provider, name)
        };

    let context = ImportContext {
        project_root_override: None,
        session_id_prefix: None,
        all_projects: true,
    };

    match provider.normalize_file(path, &context) {
        Ok(events) => {
            let event_vms = presenters::present_events(&events);
            view.render_doctor_check(&file_path, &provider_name, Ok(&event_vms))?;
            Ok(())
        }
        Err(e) => {
            view.render_doctor_check(&file_path, &provider_name, Err(&e))?;
            anyhow::bail!("Validation failed");
        }
    }
}
