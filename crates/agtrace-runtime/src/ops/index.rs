use crate::{Error, Result};
use agtrace_index::{Database, LogFileRecord, ProjectRecord, SessionRecord};
use agtrace_providers::claude::{read_team_config, team_config_paths};
use agtrace_providers::{DiscoveryScope, FileHeader, ProviderAdapter, ProviderId};
use agtrace_types::{AgentKind, AgentRef, ProjectHash, RepositoryHash};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum IndexProgress {
    IncrementalHint {
        indexed_files: usize,
    },
    LogRootMissing {
        provider_name: String,
        log_root: PathBuf,
    },
    ProviderScanning {
        provider_name: String,
    },
    ProviderSessionCount {
        provider_name: String,
        count: usize,
        project_hash: String,
        all_projects: bool,
    },
    SessionRegistered {
        session_id: String,
    },
    Completed {
        total_sessions: usize,
        scanned_files: usize,
        skipped_files: usize,
    },
}

/// Builds the session index (schema v7) from file headers only: `Provider::discover`
/// lists the agent files with their headers, `Provider::read_snippet` reads the first
/// prompt of new/changed session owners. No file is fully parsed.
pub struct IndexService<'a> {
    db: &'a Database,
    providers: Vec<(ProviderAdapter, PathBuf)>,
}

impl<'a> IndexService<'a> {
    pub fn new(db: &'a Database, providers: Vec<(ProviderAdapter, PathBuf)>) -> Self {
        Self { db, providers }
    }

    pub fn run<F>(
        &self,
        scope: agtrace_types::ProjectScope,
        force: bool,
        mut on_progress: F,
    ) -> Result<()>
    where
        F: FnMut(IndexProgress),
    {
        let indexed_files = if force {
            HashSet::new()
        } else {
            self.db
                .get_all_log_files()?
                .into_iter()
                .filter_map(|f| {
                    if should_skip_indexed_file(&f) {
                        Some(f.path)
                    } else {
                        None
                    }
                })
                .collect::<HashSet<_>>()
        };

        if !force {
            on_progress(IndexProgress::IncrementalHint {
                indexed_files: indexed_files.len(),
            });
        }

        let mut total_sessions = 0;
        let mut scanned_files = 0;
        let mut skipped_files = 0;

        // Cache repository_hash per project_root to avoid repeated git subprocess calls
        let mut repository_hash_cache: HashMap<PathBuf, Option<RepositoryHash>> = HashMap::new();

        for (adapter, log_root) in &self.providers {
            let provider_name = adapter.id();

            if !log_root.exists() {
                on_progress(IndexProgress::LogRootMissing {
                    provider_name: provider_name.to_string(),
                    log_root: log_root.clone(),
                });
                continue;
            }

            on_progress(IndexProgress::ProviderScanning {
                provider_name: provider_name.to_string(),
            });

            let headers = adapter
                .provider
                .discover(&DiscoveryScope {
                    roots: Some(vec![log_root.clone()]),
                    project_root: None,
                })
                .map_err(Error::Provider)?;

            // Claude teammates are linked to their lead via the team config, when present.
            let team_leads = match adapter.provider.id() {
                ProviderId::ClaudeCode => {
                    log_root.parent().map(read_team_leads).unwrap_or_default()
                }
                ProviderId::Codex => HashMap::new(),
            };

            let mut in_scope: Vec<(FileHeader, ProjectHash)> = headers
                .into_iter()
                .filter_map(|header| {
                    let hash = project_hash_of(&header);
                    scope
                        .hash()
                        .is_none_or(|expected| expected == &hash)
                        .then_some((header, hash))
                })
                .collect();
            // Session owners first (their rows exist before their subagent files).
            in_scope.sort_by(|(a, _), (b, _)| {
                is_session_owner(&a.agent)
                    .cmp(&is_session_owner(&b.agent))
                    .reverse()
                    .then_with(|| a.agent.file.cmp(&b.agent.file))
            });

            on_progress(IndexProgress::ProviderSessionCount {
                provider_name: provider_name.to_string(),
                count: in_scope
                    .iter()
                    .filter(|(h, _)| is_session_owner(&h.agent))
                    .count(),
                project_hash: match &scope {
                    agtrace_types::ProjectScope::All => "<all>".to_string(),
                    agtrace_types::ProjectScope::Specific(hash) => hash.to_string(),
                },
                all_projects: matches!(scope, agtrace_types::ProjectScope::All),
            });

            for (header, project_hash) in in_scope {
                let agent = &header.agent;
                let path = agent.file.display().to_string();
                if !force && indexed_files.contains(&path) {
                    skipped_files += 1;
                    continue;
                }

                let project_root = header.project_cwd.clone().or_else(|| agent.cwd.clone());
                let repository_hash = project_root.as_ref().and_then(|root| {
                    repository_hash_cache
                        .entry(root.clone())
                        .or_insert_with(|| agtrace_core::repository_hash_from_path(root))
                        .clone()
                });
                self.db.insert_or_update_project(&ProjectRecord {
                    hash: project_hash.clone(),
                    root_path: project_root
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string()),
                    last_scanned_at: Some(chrono::Utc::now().to_rfc3339()),
                })?;

                let session_id = agent.native_session_id.clone();
                if is_session_owner(agent) {
                    on_progress(IndexProgress::SessionRegistered {
                        session_id: session_id.clone(),
                    });
                    let lead = agent
                        .team
                        .as_ref()
                        .filter(|_| agent.kind == AgentKind::Teammate)
                        .and_then(|team| team_leads.get(team))
                        .filter(|lead| **lead != session_id)
                        .cloned();
                    let (parent_session_id, root_session_id) = match agent.provider {
                        ProviderId::Codex => (
                            agent
                                .parent
                                .as_ref()
                                .map(|p| p.native_session_id().to_string()),
                            Some(agent.root.native_session_id().to_string()),
                        ),
                        ProviderId::ClaudeCode => (lead.clone(), lead),
                    };
                    self.db.insert_or_update_session(&SessionRecord {
                        id: session_id.clone(),
                        project_hash,
                        repository_hash,
                        provider: provider_name.to_string(),
                        start_ts: agent
                            .started_at
                            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)),
                        end_ts: None,
                        snippet: adapter.provider.read_snippet(&agent.file),
                        is_valid: true,
                        agent_kind: agent_kind_name(agent.kind).to_string(),
                        agent_name: agent.name.clone(),
                        agent_path: agent.path.clone(),
                        team_name: agent.team.clone(),
                        root_session_id,
                        parent_session_id,
                        spawn_call_id: agent.spawn_call_id.clone(),
                    })?;
                    total_sessions += 1;
                }

                let meta = std::fs::metadata(&agent.file).ok();
                self.db.insert_or_update_log_file(&LogFileRecord {
                    path,
                    session_id,
                    role: file_role(agent).to_string(),
                    agent_id: agent.id.as_str().to_string(),
                    agent_name: agent.name.clone(),
                    spawn_call_id: agent.spawn_call_id.clone(),
                    file_size: meta.as_ref().map(|m| m.len() as i64),
                    mod_time: meta
                        .and_then(|m| m.modified().ok())
                        .map(|t| format!("{:?}", t)),
                })?;
                scanned_files += 1;
            }
        }

        on_progress(IndexProgress::Completed {
            total_sessions,
            scanned_files,
            skipped_files,
        });

        Ok(())
    }
}

/// Agents that own a `sessions` row: every agent except Claude subagents / forks,
/// which live under their parent session.
fn is_session_owner(agent: &AgentRef) -> bool {
    !agent.id.is_claude_subagent()
}

/// `log_files.role`: `main` for session owners, else `subagent` / `fork`.
fn file_role(agent: &AgentRef) -> &'static str {
    match (is_session_owner(agent), agent.kind) {
        (true, _) => "main",
        (false, AgentKind::Fork) => "fork",
        (false, _) => "subagent",
    }
}

/// `sessions.agent_kind`.
fn agent_kind_name(kind: AgentKind) -> &'static str {
    match kind {
        AgentKind::Main => "main",
        AgentKind::Teammate => "teammate",
        AgentKind::Subagent => "subagent",
        AgentKind::Fork => "fork",
        AgentKind::CodexThread => "codex_thread",
    }
}

fn project_hash_of(header: &FileHeader) -> ProjectHash {
    match header.project_cwd.as_ref().or(header.agent.cwd.as_ref()) {
        Some(root) => agtrace_core::project_hash_from_root(&root.to_string_lossy()),
        // Unique hash from the log path for files without a cwd.
        None => agtrace_core::project_hash_from_log_path(&header.agent.file),
    }
}

/// `team name -> leadSessionId` from `<claude home>/teams/*/config.json`.
fn read_team_leads(claude_home: &Path) -> HashMap<String, String> {
    team_config_paths(claude_home)
        .iter()
        .filter_map(|p| read_team_config(p))
        .filter_map(|c| Some((c.name, c.lead_session_id?)))
        .collect()
}

fn should_skip_indexed_file(indexed: &LogFileRecord) -> bool {
    let path = Path::new(&indexed.path);

    if !path.exists() {
        return false;
    }

    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };

    if let Some(db_size) = indexed.file_size {
        if db_size != metadata.len() as i64 {
            return false;
        }
    } else {
        return false;
    }

    if let Some(db_mod_time) = &indexed.mod_time {
        if let Ok(fs_mod_time) = metadata.modified() {
            let fs_mod_time_str = format!("{:?}", fs_mod_time);
            if db_mod_time != &fs_mod_time_str {
                return false;
            }
        } else {
            return false;
        }
    } else {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use agtrace_testing::live_fixture::{
        CODEX_CHILD, CODEX_FORK, CODEX_ROOT, FORK_ID, LEAD_SESSION, LiveFixture, SUBAGENT_ID,
        TEAMMATE_SESSION,
    };
    use agtrace_types::{ProjectScope, SessionOrder};

    fn index_fixture(fx: &LiveFixture, db: &Database) -> (usize, usize) {
        let providers = vec![
            (ProviderAdapter::claude(), fx.claude_home().join("projects")),
            (ProviderAdapter::codex(), fx.codex_home().join("sessions")),
        ];
        let mut completed = (0, 0);
        IndexService::new(db, providers)
            .run(ProjectScope::All, false, |p| {
                if let IndexProgress::Completed {
                    total_sessions,
                    scanned_files,
                    ..
                } = p
                {
                    completed = (total_sessions, scanned_files);
                }
            })
            .unwrap();
        completed
    }

    #[test]
    fn indexes_the_fixture_workspace_from_headers() {
        let fx = LiveFixture::new(chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap()).unwrap();
        let db = Database::open_in_memory().unwrap();
        // 5 session owners (lead, teammate, 3 Codex threads); 7 files (+ subagent, fork).
        assert_eq!(index_fixture(&fx, &db), (5, 7));

        let all = db
            .list_sessions(None, None, SessionOrder::default(), None, false)
            .unwrap();
        let get = |id: &str| all.iter().find(|s| s.id == id).unwrap().clone();

        let lead = get(LEAD_SESSION);
        assert_eq!(lead.agent_kind, "main");
        assert_eq!(lead.parent_session_id, None);
        assert!(lead.snippet.is_some());

        // Teammate: linked to its lead through the team config.
        let teammate = get(TEAMMATE_SESSION);
        assert_eq!(teammate.agent_kind, "teammate");
        assert_eq!(teammate.agent_name.as_deref(), Some("audit-A"));
        assert_eq!(teammate.team_name.as_deref(), Some("session-00000001"));
        assert_eq!(teammate.parent_session_id.as_deref(), Some(LEAD_SESSION));

        // Codex: linkage from session_meta, no timestamp correlation.
        let root = get(CODEX_ROOT);
        assert_eq!(root.agent_kind, "main");
        assert_eq!(root.root_session_id.as_deref(), Some(CODEX_ROOT));
        let child = get(CODEX_CHILD);
        assert_eq!(child.agent_kind, "codex_thread");
        assert_eq!(child.parent_session_id.as_deref(), Some(CODEX_ROOT));
        assert_eq!(child.agent_path.as_deref(), Some("/root/judge"));
        assert_eq!(get(CODEX_FORK).agent_kind, "fork");

        // Top-level listing hides children; children are queryable.
        let top: Vec<String> = db
            .list_sessions(None, None, SessionOrder::default(), None, true)
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(top.len(), 2);
        let children: Vec<String> = db
            .get_child_sessions(CODEX_ROOT)
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(children.len(), 2);
        assert_eq!(db.get_child_sessions(LEAD_SESSION).unwrap().len(), 1);

        // Subagent and fork files belong to the lead session.
        let files = db.get_session_files(LEAD_SESSION).unwrap();
        let roles: Vec<(&str, &str)> = files
            .iter()
            .map(|f| (f.role.as_str(), f.agent_id.as_str()))
            .collect();
        assert_eq!(
            roles[0],
            ("main", format!("claude:{LEAD_SESSION}").as_str())
        );
        assert!(roles.contains(&(
            "subagent",
            format!("claude:{LEAD_SESSION}/{SUBAGENT_ID}").as_str()
        )));
        assert!(roles.contains(&("fork", format!("claude:{LEAD_SESSION}/{FORK_ID}").as_str())));
        let subagent = files.iter().find(|f| f.role == "subagent").unwrap();
        assert_eq!(
            subagent.spawn_call_id.as_deref(),
            Some("toolu_synthetic_spawn_async")
        );

        // Incremental: nothing changed, nothing rescanned.
        assert_eq!(index_fixture(&fx, &db), (0, 0));
    }

    #[test]
    fn agent_tree_links_subagents_teammates_and_codex_threads() {
        use crate::client::SessionHandle;
        use std::sync::{Arc, Mutex};

        let fx = LiveFixture::new(chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap()).unwrap();
        let db = Database::open_in_memory().unwrap();
        index_fixture(&fx, &db);
        let db = Arc::new(Mutex::new(db));

        let lead = SessionHandle::for_tests(LEAD_SESSION, db.clone())
            .agent_tree()
            .unwrap();
        assert_eq!(lead.agent_id, format!("claude:{LEAD_SESSION}"));
        assert_eq!(lead.kind, "main");
        let kinds: Vec<(&str, &str)> = lead
            .children
            .iter()
            .map(|c| (c.kind.as_str(), c.agent_id.as_str()))
            .collect();
        let sub = format!("claude:{LEAD_SESSION}/{SUBAGENT_ID}");
        let fork = format!("claude:{LEAD_SESSION}/{FORK_ID}");
        let mate = format!("claude:{TEAMMATE_SESSION}");
        assert!(kinds.contains(&("subagent", sub.as_str())), "{kinds:?}");
        assert!(kinds.contains(&("fork", fork.as_str())), "{kinds:?}");
        assert!(kinds.contains(&("teammate", mate.as_str())), "{kinds:?}");
        let subagent = lead.children.iter().find(|c| c.agent_id == sub).unwrap();
        assert!(subagent.spawn_call_id.is_some());

        let root = SessionHandle::for_tests(CODEX_ROOT, db)
            .agent_tree()
            .unwrap();
        assert_eq!(root.agent_id, format!("codex:{CODEX_ROOT}"));
        let ids: Vec<&str> = root.walk().iter().map(|n| n.agent_id.as_str()).collect();
        assert!(
            ids.contains(&format!("codex:{CODEX_CHILD}").as_str()),
            "{ids:?}"
        );
        assert!(
            ids.contains(&format!("codex:{CODEX_FORK}").as_str()),
            "{ids:?}"
        );
        let child = root
            .children
            .iter()
            .find(|c| c.agent_id == format!("codex:{CODEX_CHILD}"))
            .unwrap();
        assert_eq!(child.kind, "codex_thread");
        assert_eq!(child.path.as_deref(), Some("/root/judge"));
    }

    /// Parent consistency with the live view (shared `teammate_parent` rule): a
    /// teammate spawned by a subagent of the lead session sits under that subagent
    /// in the agent tree; without a known spawn it falls back to the team lead.
    #[test]
    fn agent_tree_puts_a_teammate_under_its_spawning_agent() {
        use crate::client::SessionHandle;
        use std::sync::{Arc, Mutex};

        let tree_of = |fx: &LiveFixture| {
            let db = Database::open_in_memory().unwrap();
            index_fixture(fx, &db);
            SessionHandle::for_tests(LEAD_SESSION, Arc::new(Mutex::new(db)))
                .agent_tree()
                .unwrap()
        };
        let mate = format!("claude:{TEAMMATE_SESSION}");
        let sub = format!("claude:{LEAD_SESSION}/{SUBAGENT_ID}");

        // Fixture as is: the lead's own log spawns the teammate.
        let fx = LiveFixture::new(chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap()).unwrap();
        let tree = tree_of(&fx);
        let node = tree.children.iter().find(|c| c.agent_id == mate).unwrap();
        assert_eq!(
            node.spawn_call_id.as_deref(),
            Some("toolu_synthetic_spawn_team"),
            "spawn call id comes from the spawn"
        );

        // Move the teammate spawn (Agent call + teammate_spawned result) into the
        // subagent's log: the subagent becomes the parent.
        let lead_file = fx.lead_file();
        let text = std::fs::read_to_string(&lead_file).unwrap();
        let (spawn, rest): (Vec<&str>, Vec<&str>) = text
            .lines()
            .partition(|l| l.contains("toolu_synthetic_spawn_team"));
        assert_eq!(spawn.len(), 2);
        std::fs::write(&lead_file, rest.join("\n") + "\n").unwrap();
        let sub_file = fx.subagent_file(SUBAGENT_ID);
        let mut sub_text = std::fs::read_to_string(&sub_file).unwrap();
        sub_text.push_str(&(spawn.join("\n") + "\n"));
        std::fs::write(&sub_file, sub_text).unwrap();

        let tree = tree_of(&fx);
        assert!(
            !tree.children.iter().any(|c| c.agent_id == mate),
            "{:#?}",
            tree
        );
        let sub_node = tree.children.iter().find(|c| c.agent_id == sub).unwrap();
        let node = sub_node
            .children
            .iter()
            .find(|c| c.agent_id == mate)
            .expect("teammate under the spawning subagent");
        assert_eq!(
            node.spawn_call_id.as_deref(),
            Some("toolu_synthetic_spawn_team")
        );

        // No spawn anywhere: the team lead.
        let sub_text = std::fs::read_to_string(&sub_file).unwrap();
        let kept: Vec<&str> = sub_text
            .lines()
            .filter(|l| !l.contains("toolu_synthetic_spawn_team"))
            .collect();
        std::fs::write(&sub_file, kept.join("\n") + "\n").unwrap();
        let tree = tree_of(&fx);
        assert!(tree.children.iter().any(|c| c.agent_id == mate));
    }
    /// A resumed / bg-respawned lead: the team config's `leadSessionId` is the lead's
    /// runtime session id, which names no transcript (the lead transcript's records
    /// carry it as `session_id`), and the teammate's `agent-name` record holds the
    /// lead's display name. The teammate is still in the lead's agent tree, under its
    /// own name.
    #[test]
    fn agent_tree_resolves_a_runtime_lead_session_id() {
        use crate::client::SessionHandle;
        use std::sync::{Arc, Mutex};

        const RUNTIME: &str = "00000000-0000-4000-8000-0000000000ff";
        for collide in [false, true] {
            let fx =
                LiveFixture::new(chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap()).unwrap();
            let rewrite = |path: &std::path::Path, f: &dyn Fn(String) -> String| {
                let text = std::fs::read_to_string(path).unwrap();
                std::fs::write(path, f(text)).unwrap();
            };
            let lead_file = fx.lead_file();
            rewrite(&lead_file, &|t| {
                t.replace(
                    &format!("\"session_id\":\"{LEAD_SESSION}\""),
                    &format!("\"session_id\":\"{RUNTIME}\""),
                )
            });
            let config = fx.claude_home().join("teams/session-00000001/config.json");
            rewrite(&config, &|t| t.replace(LEAD_SESSION, RUNTIME));
            rewrite(&fx.teammate_file(), &|t| {
                let mut lines: Vec<&str> = t.lines().collect();
                lines.insert(
                    1,
                    r#"{"type":"agent-name","agentName":"Lead display name"}"#,
                );
                lines.join("\n") + "\n"
            });

            if collide {
                // A teammate of another team whose transcript id equals the runtime id.
                let other = std::fs::read_to_string(fx.teammate_file())
                    .unwrap()
                    .replace(TEAMMATE_SESSION, RUNTIME)
                    .replace("session-00000001", "session-000000ee")
                    .replace("audit-A", "other-mate");
                std::fs::write(fx.project_dir().join(format!("{RUNTIME}.jsonl")), other).unwrap();
            }

            let db = Database::open_in_memory().unwrap();
            index_fixture(&fx, &db);
            let mate = db
                .list_sessions(None, None, SessionOrder::default(), None, false)
                .unwrap()
                .into_iter()
                .find(|s| s.id == TEAMMATE_SESSION)
                .unwrap();
            assert_eq!(mate.agent_name.as_deref(), Some("audit-A"));
            assert_eq!(mate.parent_session_id.as_deref(), Some(RUNTIME));

            let tree_db = Arc::new(Mutex::new(db));
            let tree = SessionHandle::for_tests(LEAD_SESSION, tree_db.clone())
                .agent_tree()
                .unwrap();
            let node = tree
                .children
                .iter()
                .find(|c| c.agent_id == format!("claude:{TEAMMATE_SESSION}"))
                .expect("teammate under the lead transcript");
            assert_eq!(node.name.as_deref(), Some("audit-A"));
            assert_eq!(
                node.spawn_call_id.as_deref(),
                Some("toolu_synthetic_spawn_team")
            );
            if collide {
                let other = SessionHandle::for_tests(RUNTIME, tree_db.clone())
                    .agent_tree()
                    .unwrap();
                assert!(
                    other.children.is_empty(),
                    "a teammate never leads: {other:#?}"
                );
            }
        }
    }
}
