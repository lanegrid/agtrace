use crate::display_model::DoctorCheckDisplay;
use crate::ui::TraceView;
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

    let display = match provider.normalize_file(path, &context) {
        Ok(events) => DoctorCheckDisplay::from_events(file_path.clone(), provider_name, events),
        Err(e) => {
            let display = DoctorCheckDisplay::from_error(file_path, provider_name, e);
            view.render_doctor_check(&display)?;
            anyhow::bail!("Validation failed");
        }
    };

    view.render_doctor_check(&display)?;
    Ok(())
}
