use crate::utils::discover_project_root;
use anyhow::Result;

pub fn handle(project_root: Option<String>) -> Result<()> {
    let project_root_path = discover_project_root(project_root.as_deref())?;
    let project_hash = crate::utils::project_hash_from_root(&project_root_path.to_string_lossy());

    println!("Project root: {}", project_root_path.display());
    println!("Project hash: {}", project_hash);
    println!();

    let config = crate::config::Config::load()?;
    println!("Detected providers:");
    for (name, provider_config) in &config.providers {
        println!(
            "  {}: {}, log_root = {}",
            name,
            if provider_config.enabled {
                "enabled"
            } else {
                "disabled"
            },
            provider_config.log_root.display()
        );
    }

    Ok(())
}
