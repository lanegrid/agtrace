use crate::presentation::v1::view_models::DisplayOptions;
use crate::presentation::v1::view_models::SessionDigestViewModel;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub enum ReportTemplate {
    Compact,
    Diagnose,
    Tools,
}

impl FromStr for ReportTemplate {
    type Err = std::convert::Infallible;

    fn from_str(template: &str) -> Result<Self, Self::Err> {
        Ok(match template {
            "diagnose" => Self::Diagnose,
            "tools" => Self::Tools,
            _ => Self::Compact,
        })
    }
}

pub fn print_diagnose(digests: &[SessionDigestViewModel]) {
    println!("## Selected Sessions (Diagnose Mode)\n");

    let mut by_reason: HashMap<String, Vec<&SessionDigestViewModel>> = HashMap::new();
    for d in digests {
        let key = d
            .selection_reason
            .as_deref()
            .unwrap_or("Other")
            .split(' ')
            .next()
            .unwrap_or("Other");
        by_reason.entry(key.to_string()).or_default().push(d);
    }

    for (category, list) in by_reason {
        println!("### {}\n", category);
        for digest in list {
            print_digest_summary(digest);
        }
        println!();
    }
}

pub fn print_tools(digests: &[SessionDigestViewModel]) {
    print_compact(digests);
}

pub fn print_compact(digests: &[SessionDigestViewModel]) {
    let _opts = DisplayOptions {
        enable_color: false,
        relative_time: false,
        truncate_text: None,
    };

    for digest in digests {
        print_digest_summary(digest);

        println!("Work:");
        // TODO: Need to render session content from digest
        // For now, just show a placeholder
        println!("  (Session content rendering requires SessionViewModel)");
        println!();
    }
}

fn print_digest_summary(digest: &SessionDigestViewModel) {
    let id_short = &digest.session_id[..8.min(digest.session_id.len())];
    let reason = digest.selection_reason.as_deref().unwrap_or("");

    println!("Session {} ({}) -- {}", id_short, digest.source, reason);

    if let Some(opening) = &digest.opening {
        println!("  Opening: \"{}\"", opening);
    }
    if let Some(activation) = &digest.activation {
        if digest.opening.as_ref() != Some(activation) {
            println!("  Activation: \"{}\"", activation);
        }
    }
}
