use crate::session_loader::{LoadOptions, SessionLoader};
use agtrace_engine::analysis::{self, AnalysisReport, Detector};
use agtrace_index::Database;
use anyhow::Result;
use owo_colors::OwoColorize;

pub fn handle(db: &Database, session_id: String, detect: String, format: String) -> Result<()> {
    let loader = SessionLoader::new(db);
    let options = LoadOptions::default();
    let events_v2 = loader.load_events_v2(&session_id, &options)?;

    let detectors: Vec<Detector> = if detect == "all" {
        Detector::all()
    } else {
        detect
            .split(',')
            .filter_map(|d| match d.trim().parse() {
                Ok(detector) => Some(detector),
                Err(e) => {
                    eprintln!("Warning: {}", e);
                    None
                }
            })
            .collect()
    };

    let report = analysis::analyze_v2(session_id.clone(), &events_v2, detectors);

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report);
    }

    Ok(())
}

fn print_report(report: &AnalysisReport) {
    println!(
        "Analysis Report for Session: {}",
        report.session_id.bright_blue()
    );

    let score_colored = if report.score >= 90 {
        format!("{}", report.score.to_string().green())
    } else if report.score >= 70 {
        format!("{}", report.score.to_string().yellow())
    } else {
        format!("{}", report.score.to_string().red())
    };

    let warning_text = if report.warnings.len() == 1 {
        "1 Warning"
    } else {
        &format!("{} Warnings", report.warnings.len())
    };

    println!("Score: {}/100 ({})", score_colored, warning_text);
    println!();

    for warning in &report.warnings {
        println!(
            "{} {} (Count: {})",
            "[WARN]".yellow(),
            warning.pattern.bold(),
            warning.count
        );
        println!("  Span: {}", warning.span);
        println!("  Insight: {}", warning.insight);
        println!();
    }

    for info in &report.info {
        println!("{} {}", "[INFO]".cyan(), info.category.bold());
        for detail in &info.details {
            println!("  - {}", detail);
        }
        println!();
    }
}
