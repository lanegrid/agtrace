use super::{CompactView, DisplayOptions};
use agtrace_engine::SessionDigest;
use std::collections::HashMap;

pub fn print_diagnose(digests: &[SessionDigest]) {
    println!("## Selected Sessions (Diagnose Mode)\n");

    let mut by_reason: HashMap<String, Vec<&SessionDigest>> = HashMap::new();
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

pub fn print_tools(digests: &[SessionDigest]) {
    print_compact(digests);
}

pub fn print_compact(digests: &[SessionDigest]) {
    let opts = DisplayOptions {
        enable_color: false,
        relative_time: false,
        truncate_text: None,
    };

    for digest in digests {
        print_digest_summary(digest);

        println!("Work:");
        let output = format!(
            "{}",
            CompactView {
                session: &digest.session,
                options: &opts,
            }
        );
        let lines: Vec<&str> = output.lines().collect();
        for line in lines.iter().take(15) {
            println!("  {}", line);
        }
        if lines.len() > 15 {
            println!("  ... ({} more lines)", lines.len() - 15);
        }
        println!();
    }
}

fn print_digest_summary(digest: &SessionDigest) {
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
