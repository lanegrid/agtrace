use crate::display_model::{NoteLevel, ProviderSchemaContent, ProviderSchemaDisplay, SchemaItem};
use owo_colors::OwoColorize;

pub fn print_provider_schema(content: &ProviderSchemaContent) {
    match content {
        ProviderSchemaContent::RustSource(source) => {
            println!("{}", source);
        }
        ProviderSchemaContent::JsonSchema(json) => {
            println!("{}", json);
        }
        ProviderSchemaContent::Display(display) => {
            print_provider_schema_text(display);
        }
    }
}

fn print_provider_schema_text(display: &ProviderSchemaDisplay) {
    println!(
        "{}",
        format!("Provider: {}", display.provider_name)
            .bright_blue()
            .bold()
    );
    println!("Schema version: {}", display.schema_version);
    println!();
    println!(
        "{}",
        format!("Root structure ({}):", display.root_description).bold()
    );

    for section in &display.sections {
        if !section.title.is_empty() {
            if section.title == "Root structure" {
                continue;
            }
            println!();
            println!("{}", section.title.bold());
        }

        for item in &section.items {
            match item {
                SchemaItem::Field { name, description } => {
                    if description.is_empty() {
                        println!("  {}", name);
                    } else {
                        println!("  {}: {}", name, description);
                    }
                }
                SchemaItem::EnumVariant { name } => {
                    println!("    - {}", name);
                }
                SchemaItem::Note { text, level } => match level {
                    NoteLevel::Info => {
                        println!("  {}", text);
                    }
                    NoteLevel::Warning => {
                        println!("{}", text.yellow().bold());
                        println!("  {}", text);
                    }
                },
            }
        }
    }
}
