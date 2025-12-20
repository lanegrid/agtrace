use super::traits::{DiagnosticView, SessionView, SystemView, WatchView};
use crate::presentation::formatters::session_list::SessionEntry;
use crate::presentation::formatters::token::TokenUsageView;
use crate::presentation::formatters::SessionListView;
use crate::presentation::view_models::{
    CorpusStats, DiagnoseResultViewModel, DisplayOptions, DoctorCheckResultViewModel,
    EventPayloadViewModel, EventViewModel, GuidanceContext, IndexEvent, InitRenderEvent,
    InspectContent, InspectDisplay, LabStatsViewModel, ProjectSummary, ProviderConfigSummary,
    ProviderSetResult, RawFileContent, ReactionViewModel, SessionDigestViewModel,
    SessionListEntryViewModel, SessionViewModel, StreamStateViewModel, WatchStart, WatchSummary,
};
use crate::presentation::views::ReportTemplate;
use crate::presentation::views::{CompactSessionView, EventView, TimelineView};
use crate::types::OutputFormat;
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
            InitRenderEvent::Header => crate::presentation::formatters::init::print_init_header(),
            InitRenderEvent::Step1Detecting => {
                crate::presentation::formatters::init::print_step1_detecting()
            }
            InitRenderEvent::Step1Loading => {
                crate::presentation::formatters::init::print_step1_loading()
            }
            InitRenderEvent::Step1Result(step) => {
                crate::presentation::formatters::init::print_step1_result(&step)
            }
            InitRenderEvent::Step2Header => {
                crate::presentation::formatters::init::print_step2_header()
            }
            InitRenderEvent::Step2Result { db_path } => {
                crate::presentation::formatters::init::print_step2_result(&db_path)
            }
            InitRenderEvent::Step3Header => {
                crate::presentation::formatters::init::print_step3_header()
            }
            InitRenderEvent::Step3Result(step) => {
                crate::presentation::formatters::init::print_step3_result(&step)
            }
            InitRenderEvent::Step4Header => {
                crate::presentation::formatters::init::print_step4_header()
            }
            InitRenderEvent::Step4NoSessions { all_projects } => {
                crate::presentation::formatters::init::print_step4_no_sessions(all_projects);
            }
            InitRenderEvent::NextSteps { session_id } => {
                crate::presentation::formatters::init::print_next_steps(&session_id);
            }
        }
        Ok(())
    }

    fn render_lab_export(&self, exported: usize, output_path: &Path) -> Result<()> {
        println!("Exported {} events to {}", exported, output_path.display());
        Ok(())
    }

    fn render_lab_stats(&self, stats: &LabStatsViewModel) -> Result<()> {
        println!("Analyzing {} sessions...", stats.total_sessions);

        println!("\n=== ToolCall Statistics by Provider ===");
        for provider_stats in &stats.providers {
            println!("\n{}", "=".repeat(80));
            println!("Provider: {}", provider_stats.provider_name);
            println!("{}", "=".repeat(80));
            for tool_entry in &provider_stats.tools {
                println!(
                    "\n  Tool: {} (count: {})",
                    tool_entry.tool_name, tool_entry.count
                );
                if let Some(sample) = &tool_entry.sample {
                    println!("    Input:");
                    println!("      {}", sample.arguments);
                    if let Some(result) = &sample.result {
                        println!("    Output:");
                        println!("      {}", result);
                    } else {
                        println!("    Output: (no result found)");
                    }
                }
            }
        }

        println!("\n\n{}", "=".repeat(80));
        println!("=== Tool Name → Classification Mapping ===");
        println!("{}", "=".repeat(80));

        for provider_stats in &stats.providers {
            println!("\nProvider: {}", provider_stats.provider_name);
            println!("{}", "-".repeat(80));

            let max_len = provider_stats
                .classifications
                .iter()
                .map(|c| c.tool_name.len())
                .max()
                .unwrap_or(0);

            for classification in &provider_stats.classifications {
                match (&classification.origin, &classification.kind) {
                    (Some(origin), Some(kind)) => {
                        println!(
                            "  {:width$} → Origin: {:6} Kind: {:8}",
                            classification.tool_name,
                            origin,
                            kind,
                            width = max_len
                        );
                    }
                    _ => {
                        println!(
                            "  {:width$} → (unmapped - fallback to common classifier)",
                            classification.tool_name,
                            width = max_len
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

impl SessionView for ConsoleTraceView {
    fn render_session_list(
        &self,
        sessions: &[SessionListEntryViewModel],
        format: OutputFormat,
    ) -> Result<()> {
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(sessions)?);
            }
            OutputFormat::Plain => {
                let entries: Vec<SessionEntry> = sessions
                    .iter()
                    .map(|s| SessionEntry {
                        id: s.id.clone(),
                        provider: s.provider.clone(),
                        start_ts: s.start_ts.clone(),
                        snippet: s.snippet.clone(),
                    })
                    .collect();
                print!("{}", SessionListView::from_entries(entries));
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

    fn render_session_events_json(&self, events: &[EventViewModel]) -> Result<()> {
        println!("{}", serde_json::to_string_pretty(events)?);
        Ok(())
    }

    fn render_session_compact(
        &self,
        session: &SessionViewModel,
        options: &DisplayOptions,
    ) -> Result<()> {
        print!("{}", CompactSessionView { session, options });
        Ok(())
    }

    fn render_session_timeline(
        &self,
        events: &[EventViewModel],
        _truncate: bool,
        enable_color: bool,
    ) -> Result<()> {
        let options = DisplayOptions {
            enable_color,
            relative_time: true,
            truncate_text: None,
        };
        print!(
            "{}",
            TimelineView {
                events,
                options: &options,
            }
        );
        Ok(())
    }

    fn render_session_assemble_error(&self) -> Result<()> {
        eprintln!("Failed to assemble session from events");
        Ok(())
    }

    fn render_pack_report(
        &self,
        digests: &[SessionDigestViewModel],
        template: ReportTemplate,
        pool_size: usize,
        candidate_count: usize,
    ) -> Result<()> {
        println!(
            "# Packing Report (pool: {} sessions from {} raw candidates)\n",
            pool_size, candidate_count
        );
        match template {
            ReportTemplate::Compact => crate::presentation::views::pack::print_compact(digests),
            ReportTemplate::Diagnose => crate::presentation::views::pack::print_diagnose(digests),
            ReportTemplate::Tools => crate::presentation::views::pack::print_tools(digests),
        }
        Ok(())
    }
}

impl DiagnosticView for ConsoleTraceView {
    fn render_doctor_check(&self, result: &DoctorCheckResultViewModel) -> Result<()> {
        crate::presentation::views::doctor::print_check_result_vm(result);
        Ok(())
    }

    fn render_diagnose_results(
        &self,
        results: &[DiagnoseResultViewModel],
        verbose: bool,
    ) -> Result<()> {
        crate::presentation::views::doctor::print_results(results, verbose);
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
                    let bar = crate::presentation::formatters::token::create_progress_bar(
                        usage.total_tokens as i32,
                        limit as i32,
                        20,
                        true,
                    );

                    println!(
                        "{}  {} {} {:.1}% (in: {:.1}%, out: {:.1}%) - {} used / {} tokens",
                        "📊".dimmed(),
                        "Current usage:".bright_black(),
                        bar,
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

    fn on_watch_reaction(&self, reaction: &ReactionViewModel) -> Result<()> {
        match reaction {
            ReactionViewModel::Continue => {}
            ReactionViewModel::Warn(message) => {
                eprintln!("{} {}", "⚠️  Warning:".yellow(), message);
            }
        }
        Ok(())
    }

    fn render_stream_update(
        &self,
        state: &StreamStateViewModel,
        new_events: &[EventViewModel],
    ) -> Result<()> {
        let opts = DisplayOptions {
            enable_color: true,
            relative_time: false,
            truncate_text: None,
        };

        for event in new_events {
            let event_view = EventView {
                event,
                options: &opts,
                session_start: Some(state.start_time),
                turn_context: state.turn_count,
            };

            // EventView returns Ok(()) for events that shouldn't be displayed
            // We need to format to a string to check if it produced output
            let formatted = format!("{}", event_view);
            if !formatted.is_empty() {
                println!("{}", formatted);
            }

            if matches!(event.payload, EventPayloadViewModel::TokenUsage { .. }) {
                let token_view = TokenUsageView::from_usage_data(
                    state.current_usage.fresh_input,
                    state.current_usage.cache_creation,
                    state.current_usage.cache_read,
                    state.current_usage.output,
                    state.current_reasoning_tokens,
                    state.model.clone(),
                    state.token_limit,
                    state.compaction_buffer_pct,
                    opts.clone(),
                );
                println!();
                print!("{}", token_view);
            }
        }
        Ok(())
    }
}
