use super::{Cli, Commands, ProvidersCommand};
use super::import::{import_vendor_logs, count_unique_sessions, count_claude_sessions, count_codex_sessions, count_gemini_sessions};
use super::output::{print_sessions_table, print_events_timeline, print_stats, write_jsonl, write_csv};
use crate::model::*;
use crate::storage::Storage;
use crate::utils::discover_project_root;
use anyhow::Result;
use std::path::PathBuf;

pub fn run(cli: Cli) -> Result<()> {
    let data_dir = expand_tilde(&cli.data_dir);
    let storage = Storage::new(data_dir);

    match cli.command {
        Commands::Import {
            source,
            root,
            project_root,
            session_id_prefix,
            dry_run,
            out_jsonl,
        } => {
            let events = import_vendor_logs(&source, root.as_ref(), project_root.as_deref(), session_id_prefix.as_deref(), cli.all_projects)?;

            if dry_run {
                println!("Dry run: Would import {} events from {} sessions",
                    events.len(),
                    count_unique_sessions(&events));
            } else {
                storage.save_events(&events)?;
                println!("Imported {} events from {} sessions",
                    events.len(),
                    count_unique_sessions(&events));
            }

            if let Some(out_path) = out_jsonl {
                write_jsonl(&out_path, &events)?;
                println!("Wrote events to {}", out_path.display());
            }
        }

        Commands::List {
            project_hash,
            source,
            limit,
            since: _,
            until: _,
        } => {
            // If --project-hash is specified, use it (takes precedence over --all-projects)
            // If --all-projects is specified and --project-hash is not, set all_projects = true
            // Otherwise, use current project hash
            let (effective_project_hash, all_projects) = if project_hash.is_some() {
                (project_hash.as_deref(), false)
            } else if cli.all_projects {
                (None, true)
            } else {
                // Use current project hash
                let project_root_path = discover_project_root(None)?;
                let current_project_hash = crate::utils::project_hash_from_root(&project_root_path.to_string_lossy());
                (Some(current_project_hash.leak() as &str), false)
            };

            let source_enum = source.as_deref().and_then(parse_source);
            let sessions = storage.list_sessions(effective_project_hash, source_enum, Some(limit), all_projects)?;

            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&sessions)?);
            } else {
                print_sessions_table(&sessions);
            }
        }

        Commands::Show {
            session_id,
            event_type: _,
            no_reasoning,
            no_tool,
            limit,
        } => {
            let mut events = storage.load_session_events(&session_id)?;

            if no_reasoning {
                events.retain(|e| e.event_type != EventType::Reasoning);
            }

            if no_tool {
                events.retain(|e| e.event_type != EventType::ToolCall && e.event_type != EventType::ToolResult);
            }

            if let Some(lim) = limit {
                events.truncate(lim);
            }

            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&events)?);
            } else {
                print_events_timeline(&events);
            }
        }

        Commands::Find {
            session_id,
            project_hash,
            event_id: _,
            text,
            event_type,
            limit,
        } => {
            // If --project-hash is specified, use it (takes precedence over --all-projects)
            // If --all-projects is specified and --project-hash is not, set all_projects = true
            // Otherwise, use current project hash
            let (effective_project_hash, all_projects) = if project_hash.is_some() {
                (project_hash.as_deref(), false)
            } else if cli.all_projects {
                (None, true)
            } else {
                // Use current project hash
                let project_root_path = discover_project_root(None)?;
                let current_project_hash = crate::utils::project_hash_from_root(&project_root_path.to_string_lossy());
                (Some(current_project_hash.leak() as &str), false)
            };

            let event_type_enum = event_type.as_deref().and_then(parse_event_type);
            let events = storage.find_events(
                session_id.as_deref(),
                effective_project_hash,
                text.as_deref(),
                event_type_enum,
                Some(limit),
                all_projects,
            )?;

            if cli.format == "json" {
                println!("{}", serde_json::to_string_pretty(&events)?);
            } else {
                print_events_timeline(&events);
            }
        }

        Commands::Stats {
            project_hash,
            source,
            group_by: _,
            since: _,
            until: _,
        } => {
            // If --project-hash is specified, use it (takes precedence over --all-projects)
            // If --all-projects is specified and --project-hash is not, set all_projects = true
            // Otherwise, use current project hash
            let (effective_project_hash, all_projects) = if project_hash.is_some() {
                (project_hash.as_deref(), false)
            } else if cli.all_projects {
                (None, true)
            } else {
                // Use current project hash
                let project_root_path = discover_project_root(None)?;
                let current_project_hash = crate::utils::project_hash_from_root(&project_root_path.to_string_lossy());
                (Some(current_project_hash.leak() as &str), false)
            };

            let source_enum = source.as_deref().and_then(parse_source);
            let sessions = storage.list_sessions(effective_project_hash, source_enum, None, all_projects)?;

            print_stats(&sessions);
        }

        Commands::Export {
            project_hash,
            session_id,
            source: _,
            event_type,
            since: _,
            until: _,
            out,
            format,
        } => {
            // If --project-hash is specified, use it (takes precedence over --all-projects)
            // If --all-projects is specified and --project-hash is not, set all_projects = true
            // Otherwise, use current project hash
            let (effective_project_hash, all_projects) = if project_hash.is_some() {
                (project_hash.as_deref(), false)
            } else if cli.all_projects {
                (None, true)
            } else {
                // Use current project hash
                let project_root_path = discover_project_root(None)?;
                let current_project_hash = crate::utils::project_hash_from_root(&project_root_path.to_string_lossy());
                (Some(current_project_hash.leak() as &str), false)
            };

            let event_type_enum = event_type.as_deref().and_then(parse_event_type);
            let events = storage.find_events(
                session_id.as_deref(),
                effective_project_hash,
                None,
                event_type_enum,
                None,
                all_projects,
            )?;

            match format.as_str() {
                "jsonl" => write_jsonl(&out, &events)?,
                "csv" => write_csv(&out, &events)?,
                _ => anyhow::bail!("Unsupported format: {}", format),
            }

            println!("Exported {} events to {}", events.len(), out.display());
        }

        Commands::Providers { command } => {
            match command {
                None | Some(ProvidersCommand::List) => {
                    let config = crate::config::Config::load()?;

                    if config.providers.is_empty() {
                        println!("No providers configured. Run 'agtrace providers detect' to auto-detect.");
                        return Ok(());
                    }

                    println!("{:<15} {:<10} {}", "PROVIDER", "ENABLED", "LOG_ROOT");
                    println!("{}", "-".repeat(80));

                    for (name, provider_config) in &config.providers {
                        println!("{:<15} {:<10} {}",
                            name,
                            if provider_config.enabled { "yes" } else { "no" },
                            provider_config.log_root.display());
                    }
                }

                Some(ProvidersCommand::Detect) => {
                    let config = crate::config::Config::detect_providers()?;
                    config.save()?;

                    println!("Detected {} provider(s):", config.providers.len());
                    for (name, provider_config) in &config.providers {
                        println!("  {} -> {}", name, provider_config.log_root.display());
                    }
                }

                Some(ProvidersCommand::Set {
                    provider,
                    log_root,
                    enable,
                    disable,
                }) => {
                    if enable && disable {
                        anyhow::bail!("Cannot specify both --enable and --disable");
                    }

                    let mut config = crate::config::Config::load()?;

                    let enabled = if enable {
                        true
                    } else if disable {
                        false
                    } else {
                        true
                    };

                    config.set_provider(provider.clone(), crate::config::ProviderConfig {
                        enabled,
                        log_root: log_root.clone(),
                    });

                    config.save()?;

                    println!("Set provider '{}': enabled={}, log_root={}",
                        provider, enabled, log_root.display());
                }
            }
        }

        Commands::Project { project_root } => {
            let project_root_path = discover_project_root(project_root.as_deref())?;
            let project_hash = crate::utils::project_hash_from_root(&project_root_path.to_string_lossy());

            println!("Project root: {}", project_root_path.display());
            println!("Project hash: {}", project_hash);
            println!();

            let config = crate::config::Config::load()?;
            println!("Detected providers:");
            for (name, provider_config) in &config.providers {
                println!("  {}: {}, log_root = {}",
                    name,
                    if provider_config.enabled { "enabled" } else { "disabled" },
                    provider_config.log_root.display());
            }
        }

        Commands::Status { project_root } => {
            let project_root_path = discover_project_root(project_root.as_deref())?;
            let project_hash = crate::utils::project_hash_from_root(&project_root_path.to_string_lossy());

            println!("Project root: {}", project_root_path.display());
            println!("Project hash: {}", project_hash);
            println!();

            let config = crate::config::Config::load()?;
            println!("Providers:");

            for (name, provider_config) in &config.providers {
                if !provider_config.enabled {
                    continue;
                }

                println!("  {}:", name);
                println!("    log_root: {}", provider_config.log_root.display());

                let (total, matching) = match name.as_str() {
                    "claude" => count_claude_sessions(&provider_config.log_root, &project_root_path),
                    "codex" => count_codex_sessions(&provider_config.log_root, &project_root_path),
                    "gemini" => count_gemini_sessions(&provider_config.log_root, &project_hash),
                    _ => (0, 0),
                };

                println!("    sessions detected: {}", total);
                println!("    sessions matching this project: {}", matching);
                println!();
            }
        }
    }

    Ok(())
}

fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

fn parse_source(s: &str) -> Option<Source> {
    match s {
        "claude" => Some(Source::ClaudeCode),
        "codex" => Some(Source::Codex),
        "gemini" => Some(Source::Gemini),
        _ => None,
    }
}

fn parse_event_type(s: &str) -> Option<EventType> {
    match s {
        "user_message" => Some(EventType::UserMessage),
        "assistant_message" => Some(EventType::AssistantMessage),
        "reasoning" => Some(EventType::Reasoning),
        "tool_call" => Some(EventType::ToolCall),
        "tool_result" => Some(EventType::ToolResult),
        _ => None,
    }
}
