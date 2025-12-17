use crate::display_model::ProviderSchemaContent;
use crate::types::SchemaFormat;
use crate::ui::TraceView;
use anyhow::Result;

pub fn handle(provider: String, format: SchemaFormat, view: &dyn TraceView) -> Result<()> {
    let content = ProviderSchemaContent::for_provider(&provider, format)?;
    view.render_provider_schema(&content)?;
    Ok(())
}
