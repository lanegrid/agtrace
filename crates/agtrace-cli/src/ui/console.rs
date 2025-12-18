use crate::display_model::{
    DisplayOptions, DoctorCheckDisplay, ProviderSchemaContent, SessionDisplay, TokenSummaryDisplay,
};
use crate::reactor::{Reaction, SessionState};
use crate::token_limits::TokenLimits;
use crate::types::OutputFormat;
use crate::ui::models::*;
use crate::ui::traits::{DiagnosticView, SessionView, SystemView, WatchView};
use crate::views::session::{format_token_summary, print_event};
use agtrace_engine::{DiagnoseResult, SessionDigest};
use agtrace_index::SessionSummary;
use agtrace_types::{AgentEvent, EventPayload};
use anyhow::Result;
use owo_colors::OwoColorize;
use std::path::Path;

pub struct ConsoleTraceView;

impl Default for ConsoleTraceView {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleTraceView {
    pub fn new() -> Self {
        Self
    }
}

impl SystemView for ConsoleTraceView {
    fn render_guidance(&self, context: &GuidanceContext) -> Result<()> {
        println!("agtrace - Agent behavior log analyzer\n");

        if !context.config_exists || !context.db_exists {
            println!("Get started:");
            println!("  agtrace init\n");
            println!("The init command will:");
            println!("  1. Detect and configure providers (Claude, Codex, Gemini)");
            println!("  2. Set up the database");
            println!("  3. Scan for sessions");
            println!("  4. Show your recent sessions\n");
        } else if context.session_count > 0 {
            println!("Quick commands:");
            println!("  agtrace session list              # View recent sessions");
            println!("  agtrace session show <ID>         # View a session");
            println!("  agtrace index update              # Scan for new sessions");
            println!("  agtrace doctor run                # Diagnose issues\n");
        } else {
            println!("No sessions found yet.");
            println!("\nNext steps:");
            println!("  agtrace index update              # Scan for sessions");
            println!("  agtrace index update --all-projects  # Scan all projects");
            println!("  agtrace provider list             # Check provider configuration\n");
        }

        println!("For more commands:");
        println!("  agtrace --help");
        Ok(())
    }

    fn render_provider_list(&self, providers: &[ProviderConfigSummary]) -> Result<()> {
        if providers.is_empty() {
            println!("No providers configured. Run 'agtrace provider detect' to auto-detect.");
            return Ok(());
        }

        println!("{:<15} {:<10} LOG_ROOT", "PROVIDER", "ENABLED");
        println!("{}", "-".repeat(80));

        for provider in providers {
            println!(
                "{:<15} {:<10} {}",
                provider.name,
                if provider.enabled { "yes" } else { "no" },
                provider.log_root.display()
            );
        }

        Ok(())
    }

    fn render_provider_detected(&self, providers: &[ProviderConfigSummary]) -> Result<()> {
        println!("Detected {} provider(s):", providers.len());
        for provider in providers {
            println!("  {} -> {}", provider.name, provider.log_root.display());
        }
        Ok(())
    }

    fn render_provider_set(&self, result: &ProviderSetResult) -> Result<()> {
        println!(
            "Set provider '{}': enabled={}, log_root={}",
            result.provider,
            result.enabled,
            result.log_root.display()
        );
        Ok(())
    }

    fn render_warning(&self, message: &str) -> Result<()> {
        eprintln!("{}", message);
        Ok(())
    }

    fn render_project_list(
        &self,
        current_root: &str,
        current_hash: &str,
        projects: &[ProjectSummary],
    ) -> Result<()> {
        println!("Project root: {}", current_root);
        println!("Project hash: {}", current_hash);
        println!();
        println!("Registered projects:");
        println!(
            "{:<20} {:<50} {:<10} LAST SCANNED",
            "HASH (short)", "ROOT PATH", "SESSIONS"
        );
        println!("{}", "-".repeat(120));

        for project in projects {
            let hash_short = if project.hash.len() > 16 {
                format!("{}...", &project.hash[..16])
            } else {
                project.hash.clone()
            };

            println!(
                "{:<20} {:<50} {:<10} {}",
                hash_short,
                project
                    .root_path
                    .clone()
                    .unwrap_or_else(|| "(unknown)".to_string()),
                project.session_count,
                project
                    .last_scanned
                    .clone()
                    .unwrap_or_else(|| "(never)".to_string())
            );
        }

        Ok(())
    }

    fn render_corpus_overview(&self, stats: &CorpusStats) -> Result<()> {
        if stats.sample_size == 0 {
            println!("No sessions found.");
            return Ok(());
        }
        println!("# Corpus Overview (Sample: {} sessions)", stats.sample_size);
        println!("Total Tool Calls: {}", stats.total_tool_calls);
        println!("Total Failures: {}", stats.total_failures);
        println!(
            "Max Duration: {:.1}s",
            stats.max_duration_ms as f64 / 1000.0
        );
        println!("\nUse `agtrace pack --template diagnose` to see actionable problem sessions.");
        Ok(())
    }

    fn render_index_event(&self, event: IndexEvent) -> Result<()> {
        match event {
            IndexEvent::IncrementalHint { indexed_files } => {
                println!(
                    "Incremental scan mode: {} files already indexed",
                    indexed_files
                );
            }
            IndexEvent::LogRootMissing {
                provider_name,
                log_root,
            } => {
                println!(
                    "Warning: log_root does not exist for {}: {}",
                    provider_name,
                    log_root.display()
                );
            }
            IndexEvent::ProviderScanning { provider_name } => {
                println!("Scanning provider: {}", provider_name);
            }
            IndexEvent::ProviderSessionCount {
                provider_name: _,
                count,
                project_hash,
                all_projects,
            } => {
                println!(
                    "  Found {} sessions for project {}",
                    count,
                    if all_projects {
                        "(all)".to_string()
                    } else {
                        project_hash
                    }
                );
            }
            IndexEvent::SessionRegistered { session_id } => {
                println!("  Registered: {}", session_id);
            }
            IndexEvent::Completed {
                total_sessions,
                scanned_files,
                skipped_files,
                verbose,
            } => {
                if verbose {
                    println!(
                        "Scan complete: {} sessions, {} files scanned, {} files skipped",
                        total_sessions, scanned_files, skipped_files
                    );
                } else {
                    println!("Scan complete: {} sessions registered", total_sessions);
                }
            }
        }
        Ok(())
    }

    fn render_init_event(&self, event: InitRenderEvent) -> Result<()> {
        match event {
            InitRenderEvent::Header => crate::views::init::print_init_header(),
            InitRenderEvent::Step1Detecting => crate::views::init::print_step1_detecting(),
            InitRenderEvent::Step1Loading => crate::views::init::print_step1_loading(),
            InitRenderEvent::Step1Result(step) => crate::views::init::print_step1_result(&step),
            InitRenderEvent::Step2Header => crate::views::init::print_step2_header(),
            InitRenderEvent::Step2Result(display) => {
                crate::views::init::print_step2_result(&display)
            }
            InitRenderEvent::Step3Header => crate::views::init::print_step3_header(),
            InitRenderEvent::Step3Result(step) => crate::views::init::print_step3_result(&step),
            InitRenderEvent::Step4Header => crate::views::init::print_step4_header(),
            InitRenderEvent::Step4NoSessions { all_projects } => {
                crate::views::init::print_step4_no_sessions(all_projects);
            }
            InitRenderEvent::NextSteps { session_id } => {
                crate::views::init::print_next_steps(&session_id);
            }
        }
        Ok(())
    }

    fn render_lab_export(&self, exported: usize, output_path: &Path) -> Result<()> {
        println!("Exported {} events to {}", exported, output_path.display());
        Ok(())
    }
}

impl SessionView for ConsoleTraceView {
    fn render_session_list(&self, sessions: &[SessionSummary], format: OutputFormat) -> Result<()> {
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(sessions)?);
            }
            OutputFormat::Plain => {
                print_sessions_table(sessions);
            }
        }
        Ok(())
    }

    fn render_session_raw_files(&self, files: &[RawFileContent]) -> Result<()> {
        for file in files {
            println!("{}", file.content);
        }
        Ok(())
    }

    fn render_session_events_json(&self, events: &[AgentEvent]) -> Result<()> {
        println!("{}", serde_json::to_string_pretty(events)?);
        Ok(())
    }

    fn render_session_compact(
        &self,
        display: &SessionDisplay,
        options: &DisplayOptions,
    ) -> Result<()> {
        let lines = crate::views::session::format_compact(display, options);
        for line in lines {
            println!("{}", line);
        }
        Ok(())
    }

    fn render_session_timeline(
        &self,
        events: &[AgentEvent],
        truncate: bool,
        enable_color: bool,
    ) -> Result<()> {
        crate::views::session::print_events_timeline(events, truncate, enable_color);
        Ok(())
    }

    fn render_session_assemble_error(&self) -> Result<()> {
        eprintln!("Failed to assemble session from events");
        Ok(())
    }

    fn render_pack_report(
        &self,
        digests: &[SessionDigest],
        template: ReportTemplate,
        pool_size: usize,
        candidate_count: usize,
    ) -> Result<()> {
        println!(
            "# Packing Report (pool: {} sessions from {} raw candidates)\n",
            pool_size, candidate_count
        );
        match template {
            ReportTemplate::Compact => crate::views::pack::print_compact(digests),
            ReportTemplate::Diagnose => crate::views::pack::print_diagnose(digests),
            ReportTemplate::Tools => crate::views::pack::print_tools(digests),
        }
        Ok(())
    }
}

impl DiagnosticView for ConsoleTraceView {
    fn render_doctor_check(&self, display: &DoctorCheckDisplay) -> Result<()> {
        crate::views::doctor::print_check_result(display);
        Ok(())
    }

    fn render_diagnose_results(&self, results: &[DiagnoseResult], verbose: bool) -> Result<()> {
        crate::views::doctor::print_results(results, verbose);
        Ok(())
    }

    fn render_inspect(&self, display: &InspectDisplay) -> Result<()> {
        println!("File: {}", display.file_path);
        println!(
            "Lines: 1-{} (total: {} lines)",
            display.shown_lines.min(display.total_lines),
            display.total_lines
        );
        println!("{}", "─".repeat(40));

        for line in &display.lines {
            match &line.content {
                InspectContent::Raw(text) => println!("{:>6}  {}", line.number, text),
                InspectContent::Json(json) => println!(
                    "{:>6}  {}",
                    line.number,
                    serde_json::to_string_pretty(json)?
                ),
            }
        }

        println!("{}", "─".repeat(40));
        Ok(())
    }

    fn render_provider_schema(&self, content: &ProviderSchemaContent) -> Result<()> {
        crate::views::provider::print_provider_schema(content);
        Ok(())
    }
}

impl WatchView for ConsoleTraceView {
    fn render_watch_start(&self, start: &WatchStart) -> Result<()> {
        match start {
            WatchStart::Provider { name, log_root } => {
                println!(
                    "{} {} ({})",
                    "[👀 Watching]".bright_cyan(),
                    log_root.display(),
                    name
                );
            }
            WatchStart::Session { id, log_root } => {
                println!(
                    "{} session {} in {}",
                    "[👀 Watching]".bright_cyan(),
                    id,
                    log_root.display()
                );
            }
        }
        Ok(())
    }

    fn on_watch_attached(&self, display_name: &str) -> Result<()> {
        println!(
            "{}  {}\n",
            "✨ Attached to active session:".bright_green(),
            display_name
        );
        Ok(())
    }

    fn on_watch_initial_summary(&self, summary: &WatchSummary) -> Result<()> {
        if summary.recent_lines.is_empty()
            && summary.token_usage.is_none()
            && summary.turn_count == 0
        {
            return Ok(());
        }

        if !summary.recent_lines.is_empty() {
            println!(
                "{}  Last {} turn(s):\n",
                "📜".dimmed(),
                summary.recent_lines.len()
            );

            for line in &summary.recent_lines {
                println!("  {}", line);
            }
            println!();
        }

        if let Some(usage) = &summary.token_usage {
            if let Some(limit) = usage.limit {
                if let (Some(total_pct), Some(input_pct), Some(output_pct)) =
                    (usage.total_pct, usage.input_pct, usage.output_pct)
                {
                    let bar = create_progress_bar(total_pct);
                    let color_fn: fn(&str) -> String = if total_pct >= 95.0 {
                        |s: &str| s.red().to_string()
                    } else if total_pct >= 80.0 {
                        |s: &str| s.yellow().to_string()
                    } else {
                        |s: &str| s.green().to_string()
                    };

                    println!(
                        "{}  {} {} {:.1}% (in: {:.1}%, out: {:.1}%) - {} used / {} tokens",
                        "📊".dimmed(),
                        "Current usage:".bright_black(),
                        color_fn(&bar),
                        total_pct,
                        input_pct,
                        output_pct,
                        usage.total_tokens,
                        limit
                    );
                } else {
                    println!(
                        "{}  {} {} tokens used (limit {})",
                        "📊".dimmed(),
                        "Current usage:".bright_black(),
                        usage.total_tokens,
                        limit
                    );
                }
            } else {
                println!(
                    "{}  {} {} tokens used",
                    "📊".dimmed(),
                    "Current usage:".bright_black(),
                    usage.total_tokens
                );
            }
        }

        println!(
            "{}  {} total turns processed\n",
            "📝".dimmed(),
            summary.turn_count
        );
        Ok(())
    }

    fn on_watch_rotated(&self, old_path: &Path, new_path: &Path) -> Result<()> {
        let old_name = old_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| old_path.display().to_string());
        let new_name = new_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| new_path.display().to_string());
        println!(
            "\n{} {} → {}\n",
            "✨ Session rotated:".bright_green(),
            old_name.dimmed(),
            new_name
        );
        Ok(())
    }

    fn on_watch_waiting(&self, message: &str) -> Result<()> {
        println!("{} {}", "[⏳ Waiting]".bright_yellow(), message);
        Ok(())
    }

    fn on_watch_error(&self, message: &str, fatal: bool) -> Result<()> {
        eprintln!("{} {}", "❌ Error:".red(), message);
        if fatal {
            eprintln!("{}", "Watch stream terminated due to fatal error".red());
        }
        Ok(())
    }

    fn on_watch_orphaned(&self, orphaned: usize, total_events: usize) -> Result<()> {
        eprintln!(
            "{} {} orphaned events (pre-session noise), {} total events in file",
            "[DEBUG]".dimmed(),
            orphaned,
            total_events
        );
        Ok(())
    }

    fn on_watch_token_warning(&self, warning: &str) -> Result<()> {
        eprintln!("⚠️  Token validation warning: {}", warning);
        Ok(())
    }

    fn on_watch_reactor_error(&self, reactor_name: &str, error: &str) -> Result<()> {
        eprintln!("{} {} failed: {}", "❌ Reactor".red(), reactor_name, error);
        Ok(())
    }

    fn on_watch_reaction_error(&self, error: &str) -> Result<()> {
        eprintln!("{} {}", "❌ Reaction error:".red(), error);
        Ok(())
    }

    fn on_watch_reaction(&self, reaction: &Reaction) -> Result<()> {
        match reaction {
            Reaction::Continue => {}
            Reaction::Warn(message) => {
                eprintln!("{} {}", "⚠️  Warning:".yellow(), message);
            }
        }
        Ok(())
    }

    fn render_stream_update(&self, state: &SessionState, new_events: &[AgentEvent]) -> Result<()> {
        for event in new_events {
            print_event(event, state.turn_count, state.project_root.as_deref());

            if matches!(event.payload, EventPayload::TokenUsage(_)) {
                let token_limits = TokenLimits::new();
                let token_spec = state.model.as_ref().and_then(|m| token_limits.get_limit(m));

                let limit = state
                    .context_window_limit
                    .or_else(|| token_spec.as_ref().map(|spec| spec.total_limit));

                let compaction_buffer_pct = token_spec.map(|spec| spec.compaction_buffer_pct);

                let summary = TokenSummaryDisplay {
                    input: state.total_input_side_tokens(),
                    output: state.total_output_side_tokens(),
                    cache_creation: state.current_usage.cache_creation.0,
                    cache_read: state.current_usage.cache_read.0,
                    total: state.total_context_window_tokens(),
                    limit,
                    model: state.model.clone(),
                    compaction_buffer_pct,
                };

                let opts = DisplayOptions {
                    enable_color: true,
                    relative_time: false,
                    truncate_text: None,
                };

                println!();
                for line in format_token_summary(&summary, &opts) {
                    println!("{}", line);
                }
            }
        }
        Ok(())
    }
}

fn print_sessions_table(sessions: &[SessionSummary]) {
    for session in sessions {
        let id_short = if session.id.len() > 8 {
            &session.id[..8]
        } else {
            &session.id
        };

        let time_str = session.start_ts.as_deref().unwrap_or("unknown");
        let time_display = format_relative_time(time_str);

        let snippet = session.snippet.as_deref().unwrap_or("");
        let snippet_display = truncate_for_display(snippet, 80);

        let provider_display = match session.provider.as_str() {
            "claude_code" => format!("{}", session.provider.blue()),
            "codex" => format!("{}", session.provider.green()),
            "gemini" => format!("{}", session.provider.red()),
            _ => session.provider.clone(),
        };

        let snippet_final = if snippet_display.is_empty() {
            format!("{}", "[empty]".bright_black())
        } else {
            snippet_display
        };

        println!(
            "{} {} {} {}",
            time_display.bright_black(),
            id_short.yellow(),
            provider_display,
            snippet_final
        );
    }
}

fn truncate_for_display(s: &str, max_chars: usize) -> String {
    let normalized = s
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let cleaned = normalized
        .trim_start_matches("<command-name>/clear</command-name>")
        .trim_start_matches("<command-message>clear</command-message>")
        .trim()
        .to_string();

    if cleaned.chars().count() <= max_chars {
        cleaned
    } else {
        let truncated: String = cleaned.chars().take(max_chars - 3).collect();
        format!("{}...", truncated)
    }
}

fn format_relative_time(ts: &str) -> String {
    use chrono::{DateTime, Utc};

    let parsed = match DateTime::parse_from_rfc3339(ts) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => return ts.to_string(),
    };

    let now = Utc::now();
    let duration = now.signed_duration_since(parsed);

    let seconds = duration.num_seconds();
    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();

    if seconds < 60 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{} min ago", minutes)
    } else if hours < 24 {
        format!("{} hours ago", hours)
    } else if days == 1 {
        "yesterday".to_string()
    } else if days < 7 {
        format!("{} days ago", days)
    } else if days < 30 {
        let weeks = days / 7;
        format!("{} weeks ago", weeks)
    } else if days < 365 {
        let months = days / 30;
        format!("{} months ago", months)
    } else {
        let years = days / 365;
        format!("{} years ago", years)
    }
}

fn create_progress_bar(percentage: f64) -> String {
    let bar_width = 20;
    let filled = ((percentage / 100.0) * bar_width as f64) as usize;
    let filled = filled.min(bar_width);
    let empty = bar_width - filled;

    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
