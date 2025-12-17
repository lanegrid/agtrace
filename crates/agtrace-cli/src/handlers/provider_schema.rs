use crate::display_model::ProviderSchemaContent;
use crate::types::SchemaFormat;
use crate::views::provider::print_provider_schema;
use anyhow::Result;

pub fn handle(provider: String, format: SchemaFormat) -> Result<()> {
    let content = ProviderSchemaContent::for_provider(&provider, format)?;
    print_provider_schema(&content);
    Ok(())
}
