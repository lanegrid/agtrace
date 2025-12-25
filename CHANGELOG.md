# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

### Documentation

- Improve README ([ee66d7a](https://github.com/lanegrid/agtrace/commit/ee66d7abb984fc6a9abb0823138887b3ae52301e))


## [0.1.1] - 2025-12-24

### Bug Fixes

- *(watch)* Prevent panic when session_id is shorter than 8 characters ([cb4ca5a](https://github.com/lanegrid/agtrace/commit/cb4ca5af5d6033b3d27d24eaac453101064b5e13))


### Build

- Remove crates.io publishing from workflow (npm-only distribution) ([6da39e6](https://github.com/lanegrid/agtrace/commit/6da39e6b3c4a39d712375e0495295e44330bf3d6))


### Refactor

- Restore workspace-managed internal dependencies and bump to v0.1.1 ([93f80bc](https://github.com/lanegrid/agtrace/commit/93f80bc30da9c01c948f086b6f68073abe2cad9c))


## [0.1.0] - 2025-12-24

### Bug Fixes

- Prioritize content type over message.role for tool_result detection in Claude normalizer ([5b2e3dd](https://github.com/lanegrid/agtrace/commit/5b2e3dd5dddff121f32fe52ea00f2af12e9eaec4))

- Add belongs_to_project and get_search_root to LogProvider trait ([06853a3](https://github.com/lanegrid/agtrace/commit/06853a3cca85b774393be3d46a944fc2ad9554f6))

- Duplicate import ([e2cd7f4](https://github.com/lanegrid/agtrace/commit/e2cd7f4efbb81d0f3aceae0e45670cb8932886ea))

- Improve header extraction to populate timestamp and snippet fields correctly ([29ed7b2](https://github.com/lanegrid/agtrace/commit/29ed7b288826fc44275b123d5a1fd3b0eadeec74))

- Skip meta messages in snippet extraction and fix UTF-8 truncation panic ([54ddced](https://github.com/lanegrid/agtrace/commit/54ddced7334bb2b8ebc53289aa829c61e0ed0666))

- Skip meta-related messages and their descendants in snippet extraction ([0502b08](https://github.com/lanegrid/agtrace/commit/0502b08339264cff7bb54c4c7e7f4dcd43f82ac0))

- Improve bash command grouping by comparing full strings instead of truncated display strings ([3a0301d](https://github.com/lanegrid/agtrace/commit/3a0301d203cf001b9a3ca464948615c322e3ded3))

- Remove unused write_jsonl and write_csv functions ([a90fd1f](https://github.com/lanegrid/agtrace/commit/a90fd1f654969a77f07014cbb07815b2bf033a31))

- Remove duplicate snapshots (activity->engine, provider->providers) ([7b3beca](https://github.com/lanegrid/agtrace/commit/7b3becae9732bbd551782e429811fa5e0491de6b))

- Implement FromStr trait for Detector and ExportStrategy ([c806c79](https://github.com/lanegrid/agtrace/commit/c806c79f9554e91eaaf12fc24afa012887041d12))

- Resolve all clippy warnings ([0e827a8](https://github.com/lanegrid/agtrace/commit/0e827a849a8fb785883a126d9e02eb3655039698))

- Handle multibyte characters correctly in truncate_string ([70b0e77](https://github.com/lanegrid/agtrace/commit/70b0e7778f2ddd5a68ffc95cc9edf3c8c5d9066a))

- Add claude_code provider name matching for v2 normalization ([5ba8c58](https://github.com/lanegrid/agtrace/commit/5ba8c58ff1b572539ac809d561df6ad262214bcd))

- Standardize provider naming to 'claude_code' and skip Gemini logs.json files ([3fd64e6](https://github.com/lanegrid/agtrace/commit/3fd64e6d5f38f11ded4ac628dae02272cbed0c73))

- Handle broken pipe gracefully when piping to head/less ([c3460bf](https://github.com/lanegrid/agtrace/commit/c3460bf1ea1fa289b503e923d77dce5a7f80dbfe))

- Update test to use build_spans_from_events instead of build_spans_from_v2 ([e5bfd14](https://github.com/lanegrid/agtrace/commit/e5bfd14881794f9520911e88ed365af807fe0b6e))

- Eliminate Codex ghost steps by deduplicating event_msg and response_item events ([2c4b5f5](https://github.com/lanegrid/agtrace/commit/2c4b5f507ef8a62817ade185f72248f4ca9750aa))

- Prevent token double-counting and tool call duplication using max strategy ([ed01ecb](https://github.com/lanegrid/agtrace/commit/ed01ecba54db345aaacac31c5b7b2731a44260bf))

- Improve error handling and document parse_line provider extension ([e81919e](https://github.com/lanegrid/agtrace/commit/e81919e20d87f9a70fdb929662a164f075b4f7a5))

- Keep SessionWatcher alive to maintain file system monitoring ([a599345](https://github.com/lanegrid/agtrace/commit/a5993454002829d52e1fdca43f681d93f92b2a0f))

- Replace line-by-line parsing with full file re-normalization in watch mode ([df3311d](https://github.com/lanegrid/agtrace/commit/df3311dc859662d053f22f3f1ad310a10df8fd75))

- Handle multibyte characters correctly in truncate function ([b9fc4d9](https://github.com/lanegrid/agtrace/commit/b9fc4d983dc62473f71c8a25f234d6959e214bc6))

- Switch to PollWatcher for reliable Codex file change detection ([b8b3bca](https://github.com/lanegrid/agtrace/commit/b8b3bca579144df6faefe76f831b773cbe5e42e8))

- Improve path traversal detection to avoid false positives from consecutive dots ([c47f9c7](https://github.com/lanegrid/agtrace/commit/c47f9c7f139b8e5ad90e7da46ac599cd425e7b71))

- Use correct database filename in ExecutionContext (agtrace.db not db.sqlite) ([c68f876](https://github.com/lanegrid/agtrace/commit/c68f87651440dc8712ffa4f2de7ce7327b7166ec))

- Complete type safety migration in session_show and replace unsafe unwrap in watch ([c748324](https://github.com/lanegrid/agtrace/commit/c7483242d435c55fdcb2fe4f56f504c21b11083f))

- *(watch,init)* Emit Attached event after SessionRotated and show scan warnings in init ([a0cdf1f](https://github.com/lanegrid/agtrace/commit/a0cdf1fde234bfe8c4898e5838f89fc632e7efef))

- *(watch)* Send Update event when transitioning from waiting mode to active file ([65c56c3](https://github.com/lanegrid/agtrace/commit/65c56c324555c72bb4edda0a3f7c42532eeaae60))

- *(watch)* Load session history directly from file to avoid database dependency ([e2bd749](https://github.com/lanegrid/agtrace/commit/e2bd749139dfd7e8799843ca4de8078a04caf014))

- *(watch)* Display token usage after TokenUsage events and extract model from metadata correctly ([9cd1755](https://github.com/lanegrid/agtrace/commit/9cd1755083d3263432823275592b43c3f9c979f8))

- *(watch)* Track cache/reasoning tokens and extract model from TokenUsage events ([b81c501](https://github.com/lanegrid/agtrace/commit/b81c50164f7828f3b52e14c1b1d526baee570621))

- *(token)* Add cache_creation_input_tokens tracking to fix 99% underreporting ([f491656](https://github.com/lanegrid/agtrace/commit/f4916568988fa6dd2edeafe1b781200facd8fa08))

- *(providers)* Include raw metadata for TokenUsage events in v2 normalizers ([83bd5e8](https://github.com/lanegrid/agtrace/commit/83bd5e8fbda090e92233e99ea1e12373b04f2da9))

- Include cache_read_tokens in context window calculation ([594ef32](https://github.com/lanegrid/agtrace/commit/594ef32b1ea61bb8b9f69886b1a08f5d29250149))

- Resolve clippy warnings and update snapshots ([8c924b0](https://github.com/lanegrid/agtrace/commit/8c924b0b2cc774fe1531a9f8582e11a94d395979))

- Prevent UUID collision by indexing content blocks within same record ([173e390](https://github.com/lanegrid/agtrace/commit/173e390222b8791615ddef0b4a5c7a06536b2b19))

- Add missing stream_id field to AgentEvent initializations in tests ([6b2687a](https://github.com/lanegrid/agtrace/commit/6b2687a885d177e468d1315e1717ffeb76002b32))

- Restore watch start and attached messages in refresh UI header ([b4a844a](https://github.com/lanegrid/agtrace/commit/b4a844af17e88ecf0dba0446b23c3e6b17a7786b))

- Remove raw mode to allow Ctrl+C to work in TUI ([1566f0e](https://github.com/lanegrid/agtrace/commit/1566f0ec665630faa50ef25f6810dd4565941848))

- Correctly track turn count in timeline display to match watch output ([5cf2d9b](https://github.com/lanegrid/agtrace/commit/5cf2d9be472f0c1d8f0ce579a193a737588f760a))

- *(scripts)* Improve domain type pattern matching to avoid false positives with ViewModels ([9603bbb](https://github.com/lanegrid/agtrace/commit/9603bbb744417d74629d6ee39b4c53cc2fa55ca9))

- *(session)* Serialize stream_id as structured JSON instead of debug string ([1dfc032](https://github.com/lanegrid/agtrace/commit/1dfc032816d3175eed9590ceca009558f7cbecbd))

- Resolve clippy warnings for TUI implementation ([88e296f](https://github.com/lanegrid/agtrace/commit/88e296f99e3e94c4e9061ebc199a692f52573a0a))

- *(tui)* Implement native Ratatui rendering with proper UTF-8 handling ([a5f2211](https://github.com/lanegrid/agtrace/commit/a5f2211d1eee90bfcc397faa38d94d8681fb49e0))

- *(cli)* Update test imports for new runtime API ([e9965fc](https://github.com/lanegrid/agtrace/commit/e9965fce6462cd7cefdc01274dc2c794cba34d42))

- *(supervisor)* Use provider's extract_session_id() and can_handle() for robust session detection ([7bf3305](https://github.com/lanegrid/agtrace/commit/7bf3305a29a10d4d3ec0530134526097c97ed9cd))

- *(debug)* Use recv_timeout to allow Ctrl+C interruption ([ae7ae71](https://github.com/lanegrid/agtrace/commit/ae7ae7189f88553d24c2e6c818cb962355c0b68c))

- *(watch)* Handle SessionUpdated events for session rotation ([8872f6f](https://github.com/lanegrid/agtrace/commit/8872f6f2e4a85d6b1ff86ff4295f956f738831f8))

- *(watch)* Support watching sessions not yet indexed in database ([f005755](https://github.com/lanegrid/agtrace/commit/f005755aff9d642414d13b595401526f3a28ac80))

- *(tui)* Change compaction status text from 'Compaction ready' to 'Normal' ([142f13a](https://github.com/lanegrid/agtrace/commit/142f13ac86ab5f7d40d0d50435c8a3f6b820fa7c))

- *(tui)* Simplify compaction status to 'OK' and 'Near limit' ([baaa7c1](https://github.com/lanegrid/agtrace/commit/baaa7c1a66a3673372b9fa00d0a7c39a5310fec1))

- *(tui)* Persist current_user_message in AppState to track turns across event batches ([8d807e9](https://github.com/lanegrid/agtrace/commit/8d807e987198d46f69453192797529a08b2b430d))

- *(tui)* Prevent progress bar overflow by clamping total chars to bar width ([6d4c8b7](https://github.com/lanegrid/agtrace/commit/6d4c8b7284d1a6c3d952799e210a32656123863e))

- *(tui)* Calculate turn delta from cumulative token counts instead of summing all steps ([4c09eea](https://github.com/lanegrid/agtrace/commit/4c09eea20bd7d27cbf0c86af6a60a4ee1208c379))

- *(tui)* Calculate stacked bar segments as absolute ratios against max_context for visual consistency ([5db0f0e](https://github.com/lanegrid/agtrace/commit/5db0f0e77ddc4ed6638d2aad9dc4307633aae5b4))

- *(tui)* Make heavy threshold relative to max context (10%) instead of fixed 10k tokens ([e7b8bc6](https://github.com/lanegrid/agtrace/commit/e7b8bc6a6f7a5c5bd9147f89bad8e514b5387370))

- *(cli)* Add diagnostic error message for session show --raw when no log files found ([8548ca6](https://github.com/lanegrid/agtrace/commit/8548ca6e3fdcdfa7e0bfc533fd2758a76f52882c))

- *(runtime)* Resolve short session ID to full ID in get_raw_files for --raw support ([f9b6e92](https://github.com/lanegrid/agtrace/commit/f9b6e926105f54df01a81d06e997df8433624608))

- *(engine)* Preserve cumulative tokens when turn has no usage data ([fb63f4f](https://github.com/lanegrid/agtrace/commit/fb63f4fd647fd6f829dc9a9f312aee7eeb208a3b))

- *(tui)* Remove real-time turn creation logic for stable status transitions ([93c5f11](https://github.com/lanegrid/agtrace/commit/93c5f11c017be893926cb95e50bbe54c02d89a8a))

- *(engine)* Stabilize CURRENT TURN display by always marking last turn as active ([cbdd62d](https://github.com/lanegrid/agtrace/commit/cbdd62d8746fb1082a34ea8edcfa5924d60c2265))

- *(cli)* Remove redundant index call from session list, rely on read-through indexing ([d141131](https://github.com/lanegrid/agtrace/commit/d1411317ca21871a395e002a71baa28c2b3b55b2))

- *(cli)* Use UTF-8 safe text truncate for Japanese snippets in compact mode ([b900d2a](https://github.com/lanegrid/agtrace/commit/b900d2ae0efa1fb37f80c30df4e721c71e71b3a9))

- *(cli)* Use UTF-8 safe string truncation in v2 presentation layer ([98bc6db](https://github.com/lanegrid/agtrace/commit/98bc6dbdbb619a69c0f237c478c7d2f90891c244))

- *(cli)* Rename watch format flag to mode to avoid clap conflict with global format flag ([f13f37e](https://github.com/lanegrid/agtrace/commit/f13f37e1e14977b768d3165e19d9731aa9b373ef))

- *(cli)* Fix 4 critical bugs in presentation v2 (TUI scroll, JSON escape, zero division, truncate) ([1f80687](https://github.com/lanegrid/agtrace/commit/1f806876bfc6fdcf474b19a2c2b366119c7a57b8))

- *(cli)* Implement proper TUI shutdown mechanism with bidirectional channel communication ([d3ca25a](https://github.com/lanegrid/agtrace/commit/d3ca25a82f6047f621df7775a422f1b6f1f7e9ef))

- *(cli)* Restore v1's 3-pane layout (Dashboard + Turn History + Status Bar) ([a5fb183](https://github.com/lanegrid/agtrace/commit/a5fb1831af70a0bfb1d81b933c6f0cbd5c462e38))

- *(cli)* Batch process events on initial load to prevent TUI spam ([15e8068](https://github.com/lanegrid/agtrace/commit/15e8068d1c699cfb599caa8d5db7adf77c9e20c9))

- *(cli)* Use actual provider context limit (200K) instead of hardcoded 128K for bar width calculation ([5317ef1](https://github.com/lanegrid/agtrace/commit/5317ef15b32f15b516983f9a7c2e02a258dc2b33))

- *(cli)* Clamp context usage percentage to prevent gauge panic when context overflows ([3f8142d](https://github.com/lanegrid/agtrace/commit/3f8142d28fbafb3402e95bb480f352e626df8108))

- *(cli)* Adopt v1's bar calculation logic with proper delta ratio and scale-down overflow handling ([b330f7a](https://github.com/lanegrid/agtrace/commit/b330f7a02f998ee272378479072a81b020bb4d9a))

- *(cli)* Update session state on StreamEvent::Attached in watch_tui_v2 ([18fa115](https://github.com/lanegrid/agtrace/commit/18fa1151cea3f6e8c123e1a55565f12a4f1e49af))

- *(cli)* Properly reset event buffers on session attach/rotation in watch_tui_v2 ([6215bf8](https://github.com/lanegrid/agtrace/commit/6215bf8f5b0f6bd1c7fdb7b51a05b683083da3ed))

- *(cli)* Disable auto-attach when session ID is specified in watch v2 ([9da5889](https://github.com/lanegrid/agtrace/commit/9da5889862c22adb89015c0cf4e4eeac6bfe6dc0))

- *(runtime)* Support short session IDs in watch_session ([08e0337](https://github.com/lanegrid/agtrace/commit/08e0337e749070481f04cc8535190ce177543f97))

- *(runtime)* Use best-effort resolution in watch_session to handle unindexed sessions ([6cd3f0d](https://github.com/lanegrid/agtrace/commit/6cd3f0d5e85b575475300e7e1ca7c016e36df272))


### Build

- Configure cargo-dist for cross-platform binary distribution (npm, homebrew, shell installers) ([09bca38](https://github.com/lanegrid/agtrace/commit/09bca3898fb17f7c8ffd16eea75b94f9bb4b3f5e))

- Configure npm package as @lanegrid/agtrace while keeping crate name as agtrace-cli ([978527c](https://github.com/lanegrid/agtrace/commit/978527c3960fca3b5b6df85e6e868b49c3677380))

- Add automated publishing to npm (OIDC) and crates.io (token) ([843b4a2](https://github.com/lanegrid/agtrace/commit/843b4a2002e6505e8ee8b0067a009a9255783327))

- Allow custom workflow modifications for OIDC publish jobs ([d0ca5e7](https://github.com/lanegrid/agtrace/commit/d0ca5e779b3f64b65fdb0351d0242e9fd39d73ed))

- Add crates.io metadata and publish all workspace crates in dependency order ([205b578](https://github.com/lanegrid/agtrace/commit/205b5789bd8c6f97f222c439a2d3b95e1818cc8d))


### Documentation

- Update spec to v2.0 pointer architecture and add design docs ([73c8001](https://github.com/lanegrid/agtrace/commit/73c8001bcb5635e14a5bcf99bee7dda67419e782))

- Add PROGRESS.md for schema refactoring status ([e6b628c](https://github.com/lanegrid/agtrace/commit/e6b628ca5924d9a71d1f3c281a2df808e6b8c4f3))

- Update PROGRESS.md - all providers completed ([9b7e0aa](https://github.com/lanegrid/agtrace/commit/9b7e0aa055069e9940186f1058c5c806aa0d6404))

- Replace Japanese schema doc with English as primary SSOT ([228246d](https://github.com/lanegrid/agtrace/commit/228246d6cde0711f4432b6bf7b5aeefb3bbd5cae))

- Remove Japanese schema doc, English version is complete SSOT ([d476802](https://github.com/lanegrid/agtrace/commit/d476802db3c13248fe6d729f5466cec22670fffa))

- Update spec to v2.3 with init command and smart onboarding ([ae1f3b9](https://github.com/lanegrid/agtrace/commit/ae1f3b9d2d6c8479cede07804191b9285efba6f2))

- Simplify CLI spec to focus on design rationale (1300→91 lines) ([372cd90](https://github.com/lanegrid/agtrace/commit/372cd90271be93fda71901afdcf05b30927ca72a))

- Finalize PROGRESS.md with implementation summary ([e1d76bf](https://github.com/lanegrid/agtrace/commit/e1d76bf9c699d3c653572d73ebfdb520c99b0dd5))

- Update PROGRESS.md with Phase 7 completion summary ([064ea40](https://github.com/lanegrid/agtrace/commit/064ea40e9071cb19eeeefbb3297cf0bfaa013773))

- Update PROGRESS.md with Phase 8 completion summary ([f3d4ca5](https://github.com/lanegrid/agtrace/commit/f3d4ca532acf82f0cde43a7eafd2e530b6740d4c))

- Update progress tracking for v2 migration ([daed86d](https://github.com/lanegrid/agtrace/commit/daed86d16ebddcb86800d7a44637bf98a03c876b))

- Update progress tracking for Phase 3 completion ([3d482ab](https://github.com/lanegrid/agtrace/commit/3d482ab8151f63b0e221281a5499025ada80adcd))

- Update progress tracking for Phase 4 integration tests ([35c2719](https://github.com/lanegrid/agtrace/commit/35c2719abb92ab267cd91b37b9aaee9869aa72c7))

- Update Phase 5 progress tracking ([ae8584c](https://github.com/lanegrid/agtrace/commit/ae8584cc6b7d0014220dd80847479830b286cfdb))

- Complete refactoring summary, Phase 3 skipped (no duplication found) ([148bcfa](https://github.com/lanegrid/agtrace/commit/148bcfa6cd49855dee8e7468635ce2c01d59981f))

- Update PROGRESS.md with liveness window implementation summary ([86c3e49](https://github.com/lanegrid/agtrace/commit/86c3e49dd5ba37d94b9dbe7eafa5a33a4e78b730))

- Document watch mode event display issue and file re-normalization solution ([cdce37c](https://github.com/lanegrid/agtrace/commit/cdce37cb0f2260d5d6e1dafea94729f25247b495))

- Document watch mode file re-normalization implementation ([ff3b80d](https://github.com/lanegrid/agtrace/commit/ff3b80d44b42ad5af564b4060b3c81a6a61b591d))

- Add NOTE explaining why PollWatcher is used for Codex watch ([bc6779e](https://github.com/lanegrid/agtrace/commit/bc6779e7ae49a50537005a51859f984f62c2cfb3))

- Update PROGRESS.md with implementation summary ([c2609c8](https://github.com/lanegrid/agtrace/commit/c2609c8da6373940188f8f7bf840bca30ebf92f4))

- Add ExecutionContext refactoring progress and future plans ([fc08d0c](https://github.com/lanegrid/agtrace/commit/fc08d0cdbfc8956595b4f3ccf476eaf71f2a0cc4))

- Update PROGRESS.md and add ExecutionContext unit tests ([c9d91f2](https://github.com/lanegrid/agtrace/commit/c9d91f248d4daefc05b4855f6680e492ead2328c))

- Document critical database path bug fix in PROGRESS.md ([d565e99](https://github.com/lanegrid/agtrace/commit/d565e99112965a4e8c2ec6e55a9b3041f57b2393))

- Update PROGRESS.md with Phase 2.6 type safety completion ([0ad2f56](https://github.com/lanegrid/agtrace/commit/0ad2f56ed4f13cda721aa4f949e17c6f14b187c7))

- Update PROGRESS.md with incremental indexing completion and critical token tracking bug ([4b6daec](https://github.com/lanegrid/agtrace/commit/4b6daec472ea037df0a6de1904c0b29235c61fe5))

- Add explanatory comments for token snapshot semantics ([2af92a0](https://github.com/lanegrid/agtrace/commit/2af92a0aae002d581d68e1d91914c3b8161ccaeb))

- Add integration test best practices guide ([3c99dab](https://github.com/lanegrid/agtrace/commit/3c99dabfcdfbf2e18781d3b83982e5d0c3e26cce))

- Consolidate schema_goal.md into v2.rs NOTE comments ([63063db](https://github.com/lanegrid/agtrace/commit/63063dbfebf3ba2c704109a1327e9afe4f9ec321))

- Remove schema_goal.md after consolidation into code ([53446b0](https://github.com/lanegrid/agtrace/commit/53446b0f50b32043d336fcff5576e13654fc1ada))

- Consolidate database_schema.md rationale into db.rs NOTE comments ([0819f65](https://github.com/lanegrid/agtrace/commit/0819f65902cf55c6255b5ab14039a8fd3f250f34))

- Remove database_schema.md after consolidation into code ([a5af8f2](https://github.com/lanegrid/agtrace/commit/a5af8f2076a34fc81411b5a1957e2f3021b5af2d))

- Consolidate agtrace_cli_spec.md rationale into code NOTE comments ([d7d00cd](https://github.com/lanegrid/agtrace/commit/d7d00cd7e18764219eb06cc7ec3f865d64699377))

- Remove agtrace_cli_spec.md after consolidation into code ([658f8df](https://github.com/lanegrid/agtrace/commit/658f8df40b21ff02751c4b35f0986ce3e3a2aef9))

- Consolidate reactor_architecture.md rationale into reactor code NOTE comments ([a1634de](https://github.com/lanegrid/agtrace/commit/a1634de1aefba1133cfc97d71a89b9411c9cf3fa))

- Remove reactor_architecture.md after consolidation into code ([7eff53d](https://github.com/lanegrid/agtrace/commit/7eff53d0e1e0c2aa593a5114fc5c94103101dc44))

- Move troubleshooting of schema issues to docs and remove unnecessary docs ([2d8114a](https://github.com/lanegrid/agtrace/commit/2d8114a03b346270c031e8e5938f81f8a4be8b8e))

- Add PROGRESS.md for session completeness implementation handoff ([9f062f5](https://github.com/lanegrid/agtrace/commit/9f062f56f54ab1fd3ffe09c342d5a7f87bd64380))

- Simplify README ([81b3378](https://github.com/lanegrid/agtrace/commit/81b3378757eec243403f2c257cd1062be2a35261))

- Update ExecPlan with raw mode and display findings ([7acffed](https://github.com/lanegrid/agtrace/commit/7acffedfa0d44903611cace958aa34a77df6695d))

- Update ExecPlan with completion status and outcomes ([55dfa28](https://github.com/lanegrid/agtrace/commit/55dfa283959fe1af36cb492ef36aa9bd50188ff8))

- Remove execplan ([6ec0b5d](https://github.com/lanegrid/agtrace/commit/6ec0b5d711436dd5419484f53e4fe6aed69fe672))

- Update TUI refactoring completion status ([4fc3f79](https://github.com/lanegrid/agtrace/commit/4fc3f7900fa51b287b62792c9b61d80a64d61c4f))

- Update AGENTS.md ([a568acf](https://github.com/lanegrid/agtrace/commit/a568acf2c9da9734d20a975f347afd30b02c3392))

- *(watch)* Add TODO for manual session selection feature ([09b6d81](https://github.com/lanegrid/agtrace/commit/09b6d81ee8ebe1a58b128bd9f925ab0a7bd76a0b))

- Remove some deprecated docs ([6284c1a](https://github.com/lanegrid/agtrace/commit/6284c1aadae937b7619294ed74e4a7557f7c2504))

- *(lab)* Enhance grep help with examples and output preview ([f91a8a4](https://github.com/lanegrid/agtrace/commit/f91a8a4c6920e41c7bd72f6edab52db3e61af9e6))

- Add grep command usage rules for investigating event structures ([a2ce182](https://github.com/lanegrid/agtrace/commit/a2ce182042f08340c2cd0aadb36abb2b730e0df0))

- Update grep rules to include cargo build for development environment ([4df898c](https://github.com/lanegrid/agtrace/commit/4df898c890b0391dca5a8a6c11bf1fd5e168417d))

- Add agtrace-provider-normalization skill ([e464697](https://github.com/lanegrid/agtrace/commit/e46469787585bb38123ae9e125ded389683118b1))

- Update AGENTS.md ([0ec9316](https://github.com/lanegrid/agtrace/commit/0ec93160efac7d9e6743893c4bc1cf075d4288f7))

- Add presentation layer doc in presentation/mod.rs ([b89c21a](https://github.com/lanegrid/agtrace/commit/b89c21ab2f476d2776c5568540050ffc9688c33f))

- Improve crate descriptions to better reflect their actual roles ([cd728c0](https://github.com/lanegrid/agtrace/commit/cd728c09d60328cb17d640d23c191a6c31400b78))

- Make provider description future-proof for new providers ([fdc66d1](https://github.com/lanegrid/agtrace/commit/fdc66d1e06dd30e23044ac5d2d1f73dbd86b73ee))

- Add npm and crates.io publishing preparation guides ([033c1a7](https://github.com/lanegrid/agtrace/commit/033c1a71956ccd7be42c51e8330223ff17b7fd07))


### Features

- Add rusqlite dependency for v2.0 pointer architecture ([a9d0e63](https://github.com/lanegrid/agtrace/commit/a9d0e6315c81c9fe29c4d248c3817a1759fe115e))

- Implement v2.0 pointer-based architecture with schema-on-read ([3f415b9](https://github.com/lanegrid/agtrace/commit/3f415b9c2bcdd43ec942b4f824c044caaed274e3))

- Add typed schema for Gemini provider with snapshot tests ([6e48ad6](https://github.com/lanegrid/agtrace/commit/6e48ad6b26b60c5dfa0134b0eabeee6c20db41a2))

- Update Codex provider to use typed schema ([08127d1](https://github.com/lanegrid/agtrace/commit/08127d1d7c1fdd9511cd85e7f13d3912ff57e04f))

- Update Claude provider to use typed schema ([8707d15](https://github.com/lanegrid/agtrace/commit/8707d157532d71e09c8990b2e15f71ee3d0e974c))

- Extract tokens, file_path, and exit_code from provider logs ([7a2fdd7](https://github.com/lanegrid/agtrace/commit/7a2fdd7e5a7486b9083335e36a95f0b0742907f7))

- Filter out sidechain files (agent-*.jsonl) in view command ([bc9f96b](https://github.com/lanegrid/agtrace/commit/bc9f96b4c3fe1491261960a26e14344bf34f442e))

- Skip meta messages and descendants in both list and view commands ([6cdf284](https://github.com/lanegrid/agtrace/commit/6cdf284959c56090c3092d9ac712dc757d98f4c8))

- Add token display, session summary, and improve list snippet normalization ([d35b597](https://github.com/lanegrid/agtrace/commit/d35b597313573d6ff25467e03d35446d7280c5cd))

- Add color output, table formatting, filtering, and LLM-friendly options to CLI ([47ab8ba](https://github.com/lanegrid/agtrace/commit/47ab8bafbaa4a85b5b1b0dd260ba6ee2aee3234c))

- Remove truncation from mappers, add --short flag, and extract tool_result events ([3259067](https://github.com/lanegrid/agtrace/commit/32590675e5563e25db412b041bb30dd7b7b9c542))

- Add diagnose command and fix Codex schema compatibility ([cc13ae5](https://github.com/lanegrid/agtrace/commit/cc13ae55bef389c7fedb97113ab1159421bec78c))

- Add inspect, validate, schema commands with troubleshooting guide ([463f57a](https://github.com/lanegrid/agtrace/commit/463f57a48eca1021197e3ce72c409a71c32c4ec1))

- Add Gemini legacy format support and remove diagnose sampling ([bcb4bd1](https://github.com/lanegrid/agtrace/commit/bcb4bd1b3544754995cf7fc6220f954a58e4f11d))

- Fix schema compatibility and unify file filtering across providers ([86e0b7c](https://github.com/lanegrid/agtrace/commit/86e0b7cf2684e2256856b7d3299957c20d3ff9c2))

- Add FileOp enum and English v1 schema documentation as SSOT ([e66fe2f](https://github.com/lanegrid/agtrace/commit/e66fe2f2261a73c7623801bdfe4675d3ca7c4a29))

- Add analyze, export commands and enhance compact view with input summary ([06d8c4e](https://github.com/lanegrid/agtrace/commit/06d8c4e2a896091f03d51fb39583c98ad841abe0))

- Expand compact view message truncation from 60 to 100 chars ([3fd4b48](https://github.com/lanegrid/agtrace/commit/3fd4b481d411e6091fd97424ac8df2a7ccb10890))

- Improve truncation strategy with smart path compression and extended limits ([993ff3f](https://github.com/lanegrid/agtrace/commit/993ff3f27fae5394bd34e94b77ad0ae92863a269))

- Extend bash display to 80 chars and preserve quoted strings with smart tokenization ([4ca7ea0](https://github.com/lanegrid/agtrace/commit/4ca7ea06fe6f9f3605f032cdeecedd400b86f833))

- Add ToolName enum for type-safe tool normalization and fix duplicate events across providers ([0225916](https://github.com/lanegrid/agtrace/commit/02259166c1bef312007f5d9138e49a1d897c30a5))

- Add success/error indicators to compact view tool chains ([176e549](https://github.com/lanegrid/agtrace/commit/176e549067363ef1802ffc7bc0fd6556f1f65565))

- Add Activity model as intermediate representation layer between AgentEventV1 and view ([73452c5](https://github.com/lanegrid/agtrace/commit/73452c58a1a9fe443e990913cfdd766347625f7e))

- Add CLI compatibility tests and deprecation warning suppression ([3acc388](https://github.com/lanegrid/agtrace/commit/3acc3884fd30e72cd79e1eced92ff7701cdf234a))

- Add init command and smart guidance for first-time users ([14846b4](https://github.com/lanegrid/agtrace/commit/14846b426df3c94433d680a4de9d8dee5411da8d))

- Add Turn model for structured conversation representation ([0e51d78](https://github.com/lanegrid/agtrace/commit/0e51d78968e9be273ec55a6a9c0449a5b97c4f37))

- Implement Span IR for user-initiated work units ([359d877](https://github.com/lanegrid/agtrace/commit/359d877a7dc33c5e7dce6b17e3013ae5d31194d7))

- Add pack command with compact/diagnose/tools templates ([25440e3](https://github.com/lanegrid/agtrace/commit/25440e3bf3e6d63453f48149d44195efa702172a))

- Add corpus overview as default command ([0e424a2](https://github.com/lanegrid/agtrace/commit/0e424a2a52590af813d813de8957a87411763518))

- Implement lens-specific selection and provider balance for corpus/pack ([9ee5ddd](https://github.com/lanegrid/agtrace/commit/9ee5dddf1f741201bdfe1f9555d76abb6ddbc5e4))

- Add LIFO fallback for tool result matching in turn and span ([e6eca8a](https://github.com/lanegrid/agtrace/commit/e6eca8a7f30b3d189df94a16b1d95fb0c06eab7b))

- Add source/since/until filters for session list command ([ba70493](https://github.com/lanegrid/agtrace/commit/ba70493cbbe0ad0ed836ce38f666db71291dfaa8))

- Add dynamic thresholds and noise filtering to pack command ([cad5062](https://github.com/lanegrid/agtrace/commit/cad506248d98c0c4e41f38c5a8ee506a1914c801))

- Preserve raw provider logs in event raw field without modification ([7c67580](https://github.com/lanegrid/agtrace/commit/7c675809b31bbf214a7ffa8ab04575ccb1afae88))

- Add context and policy fields to capture environment and agent constraints ([24d4de9](https://github.com/lanegrid/agtrace/commit/24d4de9049b085491a03ace03df46924cfeaa6c9))

- Add raw-only snapshot tests for all providers ([3956ade](https://github.com/lanegrid/agtrace/commit/3956adeddfdbf427990edd8bb26aa3dcc5af12d1))

- Add v2 schema types and Gemini normalization layer ([0f912f3](https://github.com/lanegrid/agtrace/commit/0f912f36047ab67db6e55c9a4fe6dd6de3dba80d))

- Add v2 normalization layers for Codex and Claude ([db30768](https://github.com/lanegrid/agtrace/commit/db30768af9d0195fd9402a44d84e99919f5f8061))

- Add v2 span engine with O(1) tool matching and sidecar token tracking ([498ea77](https://github.com/lanegrid/agtrace/commit/498ea7700b1000af034b036b7c2f1af3f7a8ebd3))

- Add normalize_*_file_v2 helpers and integration tests for Codex and Claude ([1d41f45](https://github.com/lanegrid/agtrace/commit/1d41f454d41c997abad67c37194a05709b4c2f85))

- Complete Phase 4 - switch CLI to v2 pipeline with dual-pipeline validation ([89dc427](https://github.com/lanegrid/agtrace/commit/89dc427e5874c27efdf1cd6da536afe6de5e2345))

- Add AgentSession model to engine for refined event assembly ([1d64147](https://github.com/lanegrid/agtrace/commit/1d64147ba91dc0a7543e4ac193e54834c1e46c43))

- Add assembler to build AgentSession from AgentEvent (v2) with snapshot tests ([46b1c6f](https://github.com/lanegrid/agtrace/commit/46b1c6f12720632193e3ef0c037580982790eb85))

- Implement watch command with auto-attach and smart formatting for real-time session monitoring ([dbc2fb9](https://github.com/lanegrid/agtrace/commit/dbc2fb9236b762e15c8b0133885e7306e6e37bad))

- Add liveness window detection and explicit session selection to watch command ([08913e5](https://github.com/lanegrid/agtrace/commit/08913e51bd3659dded5b09ed863661843fb3ed97))

- Add project-aware session filtering to watch command using LogProvider ([abf6d23](https://github.com/lanegrid/agtrace/commit/abf6d2358241210b7e0ced83622f1bf368209390))

- Filter out agent- prefix files in Claude provider watch mode ([f9cfabc](https://github.com/lanegrid/agtrace/commit/f9cfabcd4c11c134a23703da54f5cbe6fd1a6cb0))

- Integrate session assembly into watch command for richer context ([1173a7a](https://github.com/lanegrid/agtrace/commit/1173a7a79ea167e0afd0a9a592c56887f793e17e))

- Add Notification event type and filter empty messages ([94a9dbe](https://github.com/lanegrid/agtrace/commit/94a9dbe12ec815879e8346ce6397ffae9fd0e5f9))

- Implement event-driven reactor architecture for watch command ([ad87687](https://github.com/lanegrid/agtrace/commit/ad8768711347c8da61b4f9c070efcfc41a5d987c))

- *(cli)* Add 'sessions' alias and simplify table to one-line-per-session format ([75abc30](https://github.com/lanegrid/agtrace/commit/75abc3077f559dff2f99755889586e124adf7685))

- *(watch)* Add token usage monitoring with threshold alerts and progress bar display ([6a6c180](https://github.com/lanegrid/agtrace/commit/6a6c1802268a844d5eacb14ea6f9decbd26dbd15))

- *(watch)* Display last 5 turns when attaching to active session ([d5d6417](https://github.com/lanegrid/agtrace/commit/d5d64179c960176dd2e104727809aadfc4d9a9e6))

- *(watch)* Always display token usage on every TokenUsage event ([11c3780](https://github.com/lanegrid/agtrace/commit/11c3780721da4dc87e835310a30e8b6470817f6f))

- *(watch)* Improve first view with session snapshot and token usage summary on attach ([d83fb61](https://github.com/lanegrid/agtrace/commit/d83fb612e952b27e70c3673c4bcbfa1ceb56071d))

- *(index)* Add incremental session indexing with auto-refresh on list ([6742bd1](https://github.com/lanegrid/agtrace/commit/6742bd1abdf56d0a90d87f79c7b23ccd917c8c68))

- *(watch)* Add Claude Code style context window display with progressive detail ([36937a5](https://github.com/lanegrid/agtrace/commit/36937a5cc721f226aa8ec9afc3c8b67c6635e7fd))

- Add context window override support and fix cache_read token accounting ([da34309](https://github.com/lanegrid/agtrace/commit/da34309a25b770779788fc2c05ef19b96e7281e5))

- Implement type-safe provider-specific model limits with longest prefix matching ([e483347](https://github.com/lanegrid/agtrace/commit/e483347920f1a9dd52785d311e819208e392879d))

- Add unified SessionDisplay for type-safe session rendering ([ee8d863](https://github.com/lanegrid/agtrace/commit/ee8d863aa0b102e1541e8f5d2891bd5beaa6a2ba))

- Add descriptions to CLI commands and subcommands ([42cfc85](https://github.com/lanegrid/agtrace/commit/42cfc8535ce62a0ff18f4aef043ae15d62cc1309))

- Add intervention executor with CLI implementation and runtime integration ([730af7f](https://github.com/lanegrid/agtrace/commit/730af7f878b7ca7c0a759694ea5403370c80dc47))

- Implement deterministic UUID generation using semantic suffixes ([996a3ee](https://github.com/lanegrid/agtrace/commit/996a3ee1a9de738e7bb241e81032beeab6a69ed5))

- Add agtrace-analyzer skill and update gitignore for claude files ([214eceb](https://github.com/lanegrid/agtrace/commit/214eceb15e9b0a0d4ff7d4d0a7da647236b2ee0f))

- Add rust-quality-guardian skill and fix clippy items-after-test-module warning ([9e272af](https://github.com/lanegrid/agtrace/commit/9e272afb846bf263ef27f5362146d5eda8a4c6ee))

- Implement session completeness with sidechain support and find_session_files trait method ([68358ff](https://github.com/lanegrid/agtrace/commit/68358ff7564544c6c7db5540677b2520fbeb5b69))

- Implement session-aware watch with multi-file discovery and sidechain support ([8cdcc47](https://github.com/lanegrid/agtrace/commit/8cdcc4758017f5611c242b12e3b90ec2a9988c7c))

- Add refreshing watch UI with context window footer ([597ef53](https://github.com/lanegrid/agtrace/commit/597ef5373aa19f1c17cad37bcfe32813c70df3ff))

- Make refresh UI fill terminal height dynamically ([bb257fe](https://github.com/lanegrid/agtrace/commit/bb257fe3cd447cad7959b9a0be517a06a16cc09d))

- Add time deltas, sidechain visualization, and compaction threshold marker to watch UI ([3bf68cb](https://github.com/lanegrid/agtrace/commit/3bf68cbc7d35983e1e9098439a4815febed35d71))

- Add TUI infrastructure with crossterm (Milestone 1) ([81ae9aa](https://github.com/lanegrid/agtrace/commit/81ae9aa8ecd47aec819e2277a746b9dfa41436dd))

- Implement TUI fixed footer rendering (Milestone 2) ([e5cf675](https://github.com/lanegrid/agtrace/commit/e5cf6756e6f5913c4936e7548c13a30b19977389))

- Integrate TUI mode with auto-detection (Milestone 3) ([8022341](https://github.com/lanegrid/agtrace/commit/8022341b8131bb33f3dedbc98b13a98de85f7e53))

- Add token usage summary to show command matching watch display ([eed7485](https://github.com/lanegrid/agtrace/commit/eed7485b400c3c2b7fb5a4f58fb64ad1cc0e55b3))

- Display token usage inline at each TokenUsage event in show command ([c199da5](https://github.com/lanegrid/agtrace/commit/c199da582b72f79efa8d79905c4c3985362e1552))

- *(scripts)* Add re-export checks and backward compatibility tracking to layer rules ([77b1f0a](https://github.com/lanegrid/agtrace/commit/77b1f0a46e6ccece071711eb81b6371173e9415e))

- *(cli)* Restore --refresh option to init command ([39ad522](https://github.com/lanegrid/agtrace/commit/39ad5223f95ac087f4e1d9e883391180b242c964))

- *(tui)* Add scroll functionality with keyboard navigation (j/k, up/down) ([073d5b9](https://github.com/lanegrid/agtrace/commit/073d5b9e38bc34c4d561ea2cdea4f7b00a634ddd))

- *(runtime)* Add workspace-oriented client interface ([aef5a7a](https://github.com/lanegrid/agtrace/commit/aef5a7a431ad1191b5cd70a31cec435ff63fb06f))

- *(watch)* Implement SessionRotated with automatic session switching ([6c54d79](https://github.com/lanegrid/agtrace/commit/6c54d7965b359259ebaae6b9e953b6bb288d4549))

- Add agtrace-debug tool for event stream debugging ([d02737b](https://github.com/lanegrid/agtrace/commit/d02737b730acfd41b0e2aca6c3a3663c19407477))

- *(watch)* Auto-scan and attach to latest session on startup ([9eb3674](https://github.com/lanegrid/agtrace/commit/9eb3674345274b243943a30bf8d738bc88f6b958))

- *(runtime)* Add is_new field to SessionUpdated event and session lookup by ID ([faa5ba9](https://github.com/lanegrid/agtrace/commit/faa5ba93a4de67f583b2570d00945f197fe98d78))

- *(watch)* Enhance TUI with detailed session status and context window breakdown ([fcad46a](https://github.com/lanegrid/agtrace/commit/fcad46a10e865fed574338cbf3922e4feff3bf3b))

- *(watch)* Redesign TUI with bordered boxes, bold styling, and vibrant colors for better visual hierarchy ([782bc6b](https://github.com/lanegrid/agtrace/commit/782bc6b89e871578430135c67c529394c58059f5))

- *(tui)* Improve compaction status text to 'Plenty of room' with sparkle icon ([c4d6770](https://github.com/lanegrid/agtrace/commit/c4d67706a4290ec10b4ed27f577056f43c897a97))

- *(tui)* Refresh TUI display when attaching to new session in watch mode ([24471f9](https://github.com/lanegrid/agtrace/commit/24471f950ebd17ed7b79166acc30d3a788b87c77))

- *(tui)* Add token saturation history with stacked bar visualization (History/Delta/Void segments, 10k heavy threshold) ([bd27805](https://github.com/lanegrid/agtrace/commit/bd278050ef1fb2ca3d10f64accc4810b430db998))

- *(tui)* Add beautiful bordered UI with proper layout (AGTRACE header, LIFE gauge, SATURATION HISTORY) ([20ec6f0](https://github.com/lanegrid/agtrace/commit/20ec6f0b155df362c49f9db2dc0992ed0afa23a9))

- *(tui)* Use AgentSession to build turn usage data instead of parsing raw events ([192ca14](https://github.com/lanegrid/agtrace/commit/192ca1458c0b2e86b06e649e7571ef881fdfd2c8))

- *(tui)* Implement active turn expansion with step history and footer ([cdb5e05](https://github.com/lanegrid/agtrace/commit/cdb5e05ed226820114b7e448acf3df89ea483fb8))

- *(tui)* Enhance active turn with status line, elapsed time, and detailed step display per requirements ([2f74731](https://github.com/lanegrid/agtrace/commit/2f7473188e67898fb2ccab3d8f32ac7aba3c7df9))

- *(engine,tui)* Implement AgentStep status tracking with tool-prioritized completion logic ([f7df990](https://github.com/lanegrid/agtrace/commit/f7df9901607febf04a920d336bbc5a5a310b3fbc))

- *(tui)* Add scrollable turn history with j/k keys and auto-scroll to latest ([08ab42d](https://github.com/lanegrid/agtrace/commit/08ab42def9a617f45948a6a0e9c8b535cc24d13e))

- *(lab)* Add grep command to search event payloads across sessions ([987a768](https://github.com/lanegrid/agtrace/commit/987a768c5493de7bd52b90233435e35715c0ddf9))

- *(types)* Normalize ToolCallPayload into structured enum variants ([4b0bc0f](https://github.com/lanegrid/agtrace/commit/4b0bc0fa2d616d7f1c405b5e00675afce3b0452d))

- *(cli)* Add --raw option to lab grep to show complete AgentEvent with metadata ([dad60c3](https://github.com/lanegrid/agtrace/commit/dad60c3df1fb604241e61d623ad0310d153a70ae))

- *(types)* Add ToolCallPayload::kind() to derive ToolKind from payload variant ([e5bf576](https://github.com/lanegrid/agtrace/commit/e5bf57676125bd6948eb20eaedd81e353b5c3560))

- *(providers)* Normalize Codex apply_patch to FileWrite/FileEdit with provider-specific tool layer ([648b40b](https://github.com/lanegrid/agtrace/commit/648b40b10cdae13b99483a69608cfeac5ae3e589))

- *(providers)* Normalize Codex shell tool from array command to standard Execute format ([a40dbfe](https://github.com/lanegrid/agtrace/commit/a40dbfe9e21d9033e545da78fae98a9b48aa962f))

- *(providers)* Normalize Codex read_mcp_resource from uri to file_path for FileRead ([4907d07](https://github.com/lanegrid/agtrace/commit/4907d07add617719faf3529a09d351dd5e81983f))

- *(providers)* Add Gemini provider-specific tool types with conversion layer (Codex-level) ([edf7cb4](https://github.com/lanegrid/agtrace/commit/edf7cb44a6e4bb00496166f3002d2dc949dc28ed))

- *(providers)* Add Gemini write_todos tool type with validation layer ([f18e2c4](https://github.com/lanegrid/agtrace/commit/f18e2c441d822734e9091b08dc48a7a38ad18fd5))

- *(cli)* Enhance lab grep with regex, event filters, and better code organization ([5757f9e](https://github.com/lanegrid/agtrace/commit/5757f9ed249c14da859a27c08278efa6aba7c728))

- *(providers)* Add Claude provider-specific tool types with conversion layer ([95f70fa](https://github.com/lanegrid/agtrace/commit/95f70faae2c7f3947a5ad211181ea77517301fea))

- *(providers)* Add ProviderAdapter factory methods and public trait exports ([69dbfe1](https://github.com/lanegrid/agtrace/commit/69dbfe14e5bf9dbd4e4c827f8c55e8779f9f3179))

- *(index)* Add schema version management with auto-rebuild on mismatch ([0fde62d](https://github.com/lanegrid/agtrace/commit/0fde62d9fed1d41cb237d8bdaed7374731e009e7))

- *(runtime)* Improve error message when database is not initialized ([c428c3e](https://github.com/lanegrid/agtrace/commit/c428c3e039d71686aeebc285029fbe53d9f372af))

- *(cli)* Improve init output with provider-level aggregation and reduced noise ([5ed2c8d](https://github.com/lanegrid/agtrace/commit/5ed2c8da1c2293bacbf72e19a90885091e63daa7))

- *(cli)* Enhance session show with tree structure, progress bars, and tool folding ([2d5fb00](https://github.com/lanegrid/agtrace/commit/2d5fb0036dfb690c489c380be9bdc8137486475b))

- *(cli)* Implement Phase 1 of v2 presentation architecture with ViewMode, OutputFormat, and CreateView trait ([dafa3a4](https://github.com/lanegrid/agtrace/commit/dafa3a4f2387b33eecc21a3a07f01e2c6e380854))

- *(cli)* Implement Phase 2 with mode-specific views for session list (Minimal/Compact/Standard/Verbose) ([172de82](https://github.com/lanegrid/agtrace/commit/172de825ff4703e595b7afda29918f6810e0b1d2))

- *(cli)* Wire CLI args to ViewMode with --quiet/--compact/--verbose flags for session list ([1832668](https://github.com/lanegrid/agtrace/commit/18326681b11dde990ae3d89f73b3cbbe1191c19d))

- *(cli)* Change default ViewMode to Compact and suppress badge/hints in Minimal mode ([215dbcd](https://github.com/lanegrid/agtrace/commit/215dbcd6b9460ba7650c14054438f17b6af64b7f))

- *(cli)* Improve compact mode with short ID, relative time, and inline snippets ([cd2838f](https://github.com/lanegrid/agtrace/commit/cd2838f2d64efa6bd2e13ed6451459d0c25f6a55))

- *(cli)* Add ViewMode flags (--quiet/--compact/--verbose) to session show command ([31900ae](https://github.com/lanegrid/agtrace/commit/31900ae38077aca0be268b4d2aac130bad71989a))

- *(cli)* Implement ViewMode for all v2 views (doctor, index, project, provider) ([3727e70](https://github.com/lanegrid/agtrace/commit/3727e7071257da7ca36b3e9ead0c2d39efb6ab18))

- *(cli)* Add console streaming mode for watch command (v2 presentation layer) ([63c641b](https://github.com/lanegrid/agtrace/commit/63c641be6668d0bb866bd91612437b8fc2b43f1b))

- *(cli)* Add TUI v2 presentation layer (ViewModels, Presenters, Views) for watch command ([65c64d5](https://github.com/lanegrid/agtrace/commit/65c64d5f300d588b1f07748cc98fa256169fa19f))

- *(cli)* Add TUI v2 Renderer with event loop and keyboard handling ([dab46b0](https://github.com/lanegrid/agtrace/commit/dab46b0588e7a3bd2d0af70aa0257ca0a9438d6c))

- *(cli)* Add watch --mode tui-v2 with Handler implementation ([0c2fe3a](https://github.com/lanegrid/agtrace/commit/0c2fe3ae893297e48cf2831e21025ddd72d8e038))

- *(cli)* Improve v2 TUI with v1-style intuitive monitoring features ([28a92bd](https://github.com/lanegrid/agtrace/commit/28a92bd58668e8e32baf11b827177d198032576b))

- *(cli)* Add fixed-width bars with empty portion visualization in turn history ([f6698e7](https://github.com/lanegrid/agtrace/commit/f6698e7b32df63fd0fd0984e70021e7cb73737bc))

- *(watch)* Add dark gray background to progress bar for better visibility of remaining portion ([f6ff7a1](https://github.com/lanegrid/agtrace/commit/f6ff7a15dbb60c1db2127528b6c4d18677c37289))

- *(cli)* Improve tool display with smart path truncation and empty result handling ([b183a8d](https://github.com/lanegrid/agtrace/commit/b183a8d7f7d5d673df83748343298ff67056f749))

- *(cli)* Improve multiline text display in session views with smart truncation ([ac8ad72](https://github.com/lanegrid/agtrace/commit/ac8ad72d4c40a5b33971207252236ecb96d04afc))

- Make text in TUI wide ([81aecb5](https://github.com/lanegrid/agtrace/commit/81aecb5603f109149e11a07e3d566410755fb1bb))


### Miscellaneous Tasks

- Update CLAUDE.md ([99c45d5](https://github.com/lanegrid/agtrace/commit/99c45d5c4bb66092948f25eac7f29df2695acd38))

- Cargo fmt ([f184aa7](https://github.com/lanegrid/agtrace/commit/f184aa7a71af8a8e3472f7a95c7e7d0ccac39573))

- Remove deprecated spec file ([7a0b9e1](https://github.com/lanegrid/agtrace/commit/7a0b9e1ef0fb9c7df7de1c7d9cd4dea2f7b0ff25))

- Cargo fmt ([a63ef69](https://github.com/lanegrid/agtrace/commit/a63ef69f4bc1d00890cbab59805a278c97c56dc7))

- Exclude samples-tmp/.agtrace from git ([9634f49](https://github.com/lanegrid/agtrace/commit/9634f494480b256ed90fba602dc1ba0f5e083e53))

- Remove deprecated documents ([73b0b5f](https://github.com/lanegrid/agtrace/commit/73b0b5fcd1b41961270511943db9c7012af4a5f6))

- Cargo fmt ([5393122](https://github.com/lanegrid/agtrace/commit/5393122ef9dbe633ff091cc733cc7e67f8dc3067))

- Fix clippy warnings ([97ef10d](https://github.com/lanegrid/agtrace/commit/97ef10dae46a5ea4640d4daf5aa02a172d13b9d8))

- Remove sample files ([f5f62db](https://github.com/lanegrid/agtrace/commit/f5f62dbe59fa9b0e1163315969cd4507c5afcc9f))

- Remove deprecated scripts ([cbbb7e7](https://github.com/lanegrid/agtrace/commit/cbbb7e7d2dc92d80d79c8b369d2f89a1ad5835dc))

- Update CLAUDE.md ([937bc65](https://github.com/lanegrid/agtrace/commit/937bc6516cd0b10ab1f8b6dc96f7e78bfc401b66))

- Run cargo fmt and fix clippy warnings ([d07ca76](https://github.com/lanegrid/agtrace/commit/d07ca76d41405d9b8237287357ac64286bc8f9d6))

- Update CLAUDE.md ([8e9afa9](https://github.com/lanegrid/agtrace/commit/8e9afa9e471ade9e6ae6794706f86f8f391022d1))

- Update CLAUDE.md ([b9dbbcb](https://github.com/lanegrid/agtrace/commit/b9dbbcb4e542c425426940b26bdf96cf53340826))

- Add OUTPUT.md to gitignore ([68d5b6d](https://github.com/lanegrid/agtrace/commit/68d5b6d46dd98b0de393e4727b886174612d5337))

- Update CLAUDE.md ([e78d02b](https://github.com/lanegrid/agtrace/commit/e78d02bde31c9f099a6f1062559c7a8cd627ce19))

- Remove temporary documents ([b2f45d5](https://github.com/lanegrid/agtrace/commit/b2f45d5eb0214c1cdfa3ef53533fc985f73831e3))

- Update CLAUDE.md ([23378f9](https://github.com/lanegrid/agtrace/commit/23378f9cbd9b7977a09c778a7082fcaf0e94e63b))

- Add v2 schema plan ([052eb47](https://github.com/lanegrid/agtrace/commit/052eb474842b5ced0cf09df012f8691df09904a9))

- Remove previous PROGRESS.md ([3582853](https://github.com/lanegrid/agtrace/commit/3582853ae41eea71d5602031aff7a26de5f8aea8))

- Remove PROGRESS.md ([cd24d83](https://github.com/lanegrid/agtrace/commit/cd24d836b2326c4fb14773b067292beea5019b1f))

- Remove PROGRESS.md ([f577fdb](https://github.com/lanegrid/agtrace/commit/f577fdb5b8423627eea2381b07cbe690306525e5))

- Remove deprecated doc ([7056bb2](https://github.com/lanegrid/agtrace/commit/7056bb261bb7d879a8bd7b63dc1c31f99be7277e))

- Remove PROGRESS.md ([04b555a](https://github.com/lanegrid/agtrace/commit/04b555a39df6f5300291562ac196ec16d879263b))

- Update CLAUDE.md ([17daab2](https://github.com/lanegrid/agtrace/commit/17daab2883490ccbc9692a696f996f2a91651b76))

- Remove PROGRESS.md ([27d8c5f](https://github.com/lanegrid/agtrace/commit/27d8c5f98e71621e0b6ffdfab28142952d8e74cd))

- Remove PROGRESS.md ([6393f37](https://github.com/lanegrid/agtrace/commit/6393f3762e5398730f11be6412c1fd61b83891f2))

- Add AGENTS.md ([61bf5cb](https://github.com/lanegrid/agtrace/commit/61bf5cb03e3c77249d3ed4000f1ff8bb232e6035))

- Fmt ([58c03d3](https://github.com/lanegrid/agtrace/commit/58c03d376bfda5458d173ad3db7977f4deeb234b))

- Add example for generating engine test fixtures from provider data ([faa91c6](https://github.com/lanegrid/agtrace/commit/faa91c62dabcb0e4885f2918f284fb01a67f2822))

- Update SKILLs ([53b7ab8](https://github.com/lanegrid/agtrace/commit/53b7ab808c631d5935bafbdf80f295f4909401d4))

- Remove PROGRESS.md ([07099b0](https://github.com/lanegrid/agtrace/commit/07099b07471329571e3a6ad50bbcd000efa4d93c))

- Remove PROGRESS.md ([ff39185](https://github.com/lanegrid/agtrace/commit/ff39185b2d2b6d087744b18ffb4e9e2d67cd2506))

- Update gitignore ([8481806](https://github.com/lanegrid/agtrace/commit/84818066cc0ef36214717378253a3ba1e6db07cf))

- Remove unused .old files ([2fb74d3](https://github.com/lanegrid/agtrace/commit/2fb74d341bcb40852d52523cdd4f68ec94cf89c7))

- Remove unused encode_claude_project_dir function ([c4d4447](https://github.com/lanegrid/agtrace/commit/c4d44476601709b78cd0e90e490d1501302d678e))

- Improve some paths for tests or docs ([a2a3b99](https://github.com/lanegrid/agtrace/commit/a2a3b99a00350202c847595b1936d327e4ca1cef))

- Remove authors field from Cargo.toml files following Rust modern conventions ([c971c7c](https://github.com/lanegrid/agtrace/commit/c971c7c5dd33c9cd2c3347df05d9b8da81ae536d))

- Upgrade to Rust edition 2024 ([f304470](https://github.com/lanegrid/agtrace/commit/f30447091156cc0e34d67654bbbd3f1a4cbb0869))

- Update dependencies to latest versions ([69144ec](https://github.com/lanegrid/agtrace/commit/69144ec38d41e3c47e7deb4ad840f7074fce4c86))

- Add dual license files (MIT and Apache-2.0) ([35c70f4](https://github.com/lanegrid/agtrace/commit/35c70f4b9ffc685db6d9d2c5c1994480176d7433))

- Remove unused comfy-table dependency ([350c19d](https://github.com/lanegrid/agtrace/commit/350c19dc54f933355354220fb6867760a4c362f5))

- Clean up unused agent skill files ([a4ae2d1](https://github.com/lanegrid/agtrace/commit/a4ae2d121f8ce1c4faf0349cf49a84caf168b3e9))

- Pin Rust version 1.90.0 with mise ([dc2ec35](https://github.com/lanegrid/agtrace/commit/dc2ec3515183cbbbb2ea9ca804bd8d86b73d043d))

- Remove outdated session_id_design.md ([9ed7104](https://github.com/lanegrid/agtrace/commit/9ed71041c96a08169524c7f546776cec199e8295))

- Add rust-toolchain.toml for rustup compatibility ([3ba1438](https://github.com/lanegrid/agtrace/commit/3ba14387fcedef21785ae9956cfd40ee464f2462))

- Remove .mise.toml (rust-toolchain.toml is sufficient) ([651d1f6](https://github.com/lanegrid/agtrace/commit/651d1f6bd969e412241f5390293d8011ebd49641))

- Remove unused scripts directory ([0ca6e84](https://github.com/lanegrid/agtrace/commit/0ca6e846f20df992fd0442dd289573a2125cec8a))

- Remove unused agtrace-debug crate ([e1d06b6](https://github.com/lanegrid/agtrace/commit/e1d06b6f79c954b1906f1b061c59553e8829bd50))


### Performance

- *(runtime)* Optimize SessionOps::find to reindex only when session not found ([d25ff2d](https://github.com/lanegrid/agtrace/commit/d25ff2dbd981a0537cce836ef74106e3d929b1f5))


### Refactor

- Refactor cli ([791a4bf](https://github.com/lanegrid/agtrace/commit/791a4bfd532ed7bda7448806e6ffdec36ea0dd54))

- Refactor cli 2 ([5eb9ac9](https://github.com/lanegrid/agtrace/commit/5eb9ac957a97551fa4f7499148b223b4d99b779e))

- Refactor cli 3 ([e41b96d](https://github.com/lanegrid/agtrace/commit/e41b96d452e931548bde813993cc56d48514497b))

- Refactor cli ([ebe7d57](https://github.com/lanegrid/agtrace/commit/ebe7d5719b3669e29daa09269cbbf205af4176a1))

- Split cli.rs into modular structure (mod, commands, import, output) ([2213632](https://github.com/lanegrid/agtrace/commit/2213632f9a69ebbf50135953ddbab7bb1f3b6cee))

- Implement Clean Architecture with trait-based provider system ([5dea93b](https://github.com/lanegrid/agtrace/commit/5dea93b1059bfd78be1063315ad2fdf81c2da994))

- Split CLI commands into handlers and separate args ([41356e2](https://github.com/lanegrid/agtrace/commit/41356e275fa69c3fb57b7bc5bbfe4c59b8e1d654))

- Split claude.rs into modular structure (mod/io/mapper) ([a33aed7](https://github.com/lanegrid/agtrace/commit/a33aed75ea5d168da97088c1e1ab3741cffa8352))

- Split codex and gemini providers into modular structure ([3383d20](https://github.com/lanegrid/agtrace/commit/3383d206e7afc0e44a112a694a6d33a1b7f5eb85))

- Consolidate duplicate parsing and utility functions ([767e21b](https://github.com/lanegrid/agtrace/commit/767e21b239bdfd6f93060ba9ee517e61b57ca4d1))

- Consolidate file iteration and remove redundant count functions ([ab9875f](https://github.com/lanegrid/agtrace/commit/ab9875f8eceae0570f28e428e7e8924c724c7ade))

- Remove git root detection from project_root resolution ([361aad8](https://github.com/lanegrid/agtrace/commit/361aad8a68517769950ad09d02b0e0caba1d7c05))

- Extract metadata from file content instead of path names ([40a1617](https://github.com/lanegrid/agtrace/commit/40a1617bf12fef413e41e9881214ae2b0307cb49))

- Remove deprecated import and status commands ([a7ed359](https://github.com/lanegrid/agtrace/commit/a7ed3597442ac6081a2dff3848e05f8d7b13277c))

- Remove unused print_sessions_table function ([cf3bfee](https://github.com/lanegrid/agtrace/commit/cf3bfee08e750e3989e8e5fa806399bb6bef4f4d))

- Rename core to activity and consolidate into single file ([b38d87b](https://github.com/lanegrid/agtrace/commit/b38d87b9e1c3b6c50b67c8c4b0c99558f0e7fd04))

- Reorganize CLI commands into hierarchical namespaces with backward-compatible legacy aliases ([13265e1](https://github.com/lanegrid/agtrace/commit/13265e12cacd8139f023c54659f770e92bebfa9d))

- Establish module boundaries with types/providers/index/engine/cli structure ([b91dedc](https://github.com/lanegrid/agtrace/commit/b91dedc58a1a21d56fffcd20db800339529d9fd7))

- Remove deprecated Storage handlers and move tests to unit tests ([df39da5](https://github.com/lanegrid/agtrace/commit/df39da50ab84bc9356968207292d237677390111))

- Migrate to Cargo workspace with 5 crates (types/providers/index/engine/cli) ([99a5def](https://github.com/lanegrid/agtrace/commit/99a5def72e24c34e91122dc62c38979d91173905))

- Complete workspace migration by moving CLI code and tests to respective crates and removing root package ([8de2be6](https://github.com/lanegrid/agtrace/commit/8de2be64ba7747c1abe0ccd92981e7035b66bf88))

- Remove duplicate src/mod.rs files and unify crate entry points to lib.rs ([4281230](https://github.com/lanegrid/agtrace/commit/4281230e0cf2e15b82ee4f6256ea8bb143386953))

- Tighten public API boundaries and add workspace default-members ([342669c](https://github.com/lanegrid/agtrace/commit/342669c44aa832015097e982176afed5707c2b2f))

- Improve init UX with smart scan skip and project scope ([e478654](https://github.com/lanegrid/agtrace/commit/e478654f24fd9645efdf8a049d5c86cb1b327419))

- Move core logic from CLI to engine layer with configurable options ([8026181](https://github.com/lanegrid/agtrace/commit/8026181c3f6073cd2e95d5f0e712a93d880308dc))

- Remove deprecated CLI commands and consolidate provider handlers ([f95ce88](https://github.com/lanegrid/agtrace/commit/f95ce8844cf57a522c9266b8e462ef78a75efe7a))

- Rename handler files to match command hierarchy ([96f880f](https://github.com/lanegrid/agtrace/commit/96f880f0ea3eceafc0a80f086f14031c33c7dcbd))

- Add engine façade API, move rendering to output layer, and make display options configurable ([3b99e2a](https://github.com/lanegrid/agtrace/commit/3b99e2ab20993222d0d71c5dd08e63259cead395))

- Replace Activity model with Turn model ([3fe1692](https://github.com/lanegrid/agtrace/commit/3fe16922658491ef4a83c68e46caeef981609793))

- Skip duplicate AgentReasoning events from EventMsg ([48bfb3c](https://github.com/lanegrid/agtrace/commit/48bfb3c915013f3b0bec5adaf8e041fdce2ad12c))

- Extract format functions from compact output for pack reuse ([0594e20](https://github.com/lanegrid/agtrace/commit/0594e20883e33f945b545f2aef498b8fac23a796))

- Migrate session show compact to Span-based output ([8b86e2f](https://github.com/lanegrid/agtrace/commit/8b86e2f0a39056ad3e3f24dafaf8de5f068e0484))

- Remove provider-specific code from agtrace-types to fix layer violation ([68d3ead](https://github.com/lanegrid/agtrace/commit/68d3ead3e8d21efbf1e9a9c53895e5c8078a884e))

- Move provider-specific logic from agtrace-cli to agtrace-providers registry ([d9fd156](https://github.com/lanegrid/agtrace/commit/d9fd156122ea4adb90ea1a5d86867a600f5d113e))

- Remove unused implementations from agtrace-types and move display logic to CLI layer ([15030b2](https://github.com/lanegrid/agtrace/commit/15030b2d1a34887c49208e8b83e7af7eba6cb9df))

- Add provider_call_id and audio tokens to v2 schema per review ([acc8b16](https://github.com/lanegrid/agtrace/commit/acc8b1624a2869c02e8d78803ebd480b6ff69efd))

- Migrate LogProvider and CLI to v2 pipeline, remove v1 loading and test code ([51a5235](https://github.com/lanegrid/agtrace/commit/51a523549c3f142b760d43d14b74d9e723b18cb8))

- Complete v1 to v2 migration (95%) - delete v1 mappers, migrate analysis/export/timeline to v2 ([31995a7](https://github.com/lanegrid/agtrace/commit/31995a76a243ea435affaf8d393ef315031f256f))

- Complete v1 to v2 migration (100%) - native v2 analysis/export, delete AgentEventV1 and all v1 code ([d423e80](https://github.com/lanegrid/agtrace/commit/d423e80321b3eca6a1ce620ae5cf00f80aef857a))

- Remove unused v1 types (EventType, Role, Channel, FileOp, ToolName) - keep only ToolStatus for Span API ([71e4ad9](https://github.com/lanegrid/agtrace/commit/71e4ad965d3c9a0114b9aae5643bf4379b8c852e))

- Fix ghost steps by introducing pending call map for cross-step tool result resolution ([ae8d756](https://github.com/lanegrid/agtrace/commit/ae8d75692cdf5b973dbba51e5e18a486588c61ce))

- Change SessionSummary to be built from AgentSession instead of AgentEvent ([ef44dc2](https://github.com/lanegrid/agtrace/commit/ef44dc2fcd1b1dc117b24826995a240fbcf63b55))

- Remove Span dependency from CLI, use AgentSession and simplify SessionSummary to event counts only ([32637e4](https://github.com/lanegrid/agtrace/commit/32637e42b70c11a197d95ac066c201318c710e47))

- Separate pack analysis logic into engine layer, reduce pack.rs from 523 to 95 lines ([0544888](https://github.com/lanegrid/agtrace/commit/0544888b7430537757eac3b0a222f94e43c805c8))

- Separate doctor validation logic into engine diagnostics module, reduce doctor_run.rs from 257 to 117 lines ([6749929](https://github.com/lanegrid/agtrace/commit/6749929a8fbd887eef6fb8748f04911f680482d7))

- Separate watch logic into streaming module with SessionWatcher for better testability and extensibility ([174cd2b](https://github.com/lanegrid/agtrace/commit/174cd2bd1982a1e5a927e70836ed33898270b745))

- Use SessionUpdate fields to eliminate 5 dead_code annotations ([0e78ccc](https://github.com/lanegrid/agtrace/commit/0e78cccc86653aad694f4af7fb5e288a91ec6806))

- Remove unused into_receiver method and dead_code annotation from receiver ([dd0e94c](https://github.com/lanegrid/agtrace/commit/dd0e94c90c41df97ff57eccee30d785ad1ad19f6))

- Introduce ExecutionContext and WatchTarget to eliminate provider path detection ([e0a5c28](https://github.com/lanegrid/agtrace/commit/e0a5c2848a9c0fc4b3b926c99bd2d40db4559d12))

- Migrate index and doctor_run to ExecutionContext (Phase 1) ([f8ef9cd](https://github.com/lanegrid/agtrace/commit/f8ef9cdc0daa9d96605f21803501aae8617ec202))

- Migrate corpus_overview, pack, and project to ExecutionContext (Phase 2) ([7ddb568](https://github.com/lanegrid/agtrace/commit/7ddb568d6507f6c97b603e2e812ccfcd0cb899c5))

- Migrate init handler to ExecutionContext ([143efb5](https://github.com/lanegrid/agtrace/commit/143efb5cd406f9397c502dc79a4443fcac909b0a))

- Replace stringly-typed parameters with domain enums ([430eed4](https://github.com/lanegrid/agtrace/commit/430eed456308b292b7b9652c36395812ee4d888e))

- Complete type safety migration for provider_schema and session_list ([f90d3d1](https://github.com/lanegrid/agtrace/commit/f90d3d134e50d2218be92bd89d8f93c793b2f907))

- *(watch)* Extract testable functions and add unit tests ([8eac166](https://github.com/lanegrid/agtrace/commit/8eac166ed66f31b312463128d51e83a530c35f5f))

- *(watch)* Extract StreamEvent handlers and add comprehensive tests ([4c029f2](https://github.com/lanegrid/agtrace/commit/4c029f25555b938e4ce3a7da9421a5f432a11fe4))

- *(token)* Add type-safe token calculation to prevent cache token omission ([09f283c](https://github.com/lanegrid/agtrace/commit/09f283c8c5f059d454faaa35d039c21ab78446b6))

- *(token)* Fix fundamental misunderstanding - use snapshots not cumulative totals ([a5b54fb](https://github.com/lanegrid/agtrace/commit/a5b54fb95147c9432b36c0f8c9dd6cb233a16e0f))

- Add type-safe token usage tracking to prevent context window bugs ([f808d1d](https://github.com/lanegrid/agtrace/commit/f808d1d7f3c7c70e82023c00781bd884af1a42ec))

- Improve method naming for clarity (total -> context_window_tokens) ([a5ae7fc](https://github.com/lanegrid/agtrace/commit/a5ae7fc65818827099dfa1217006bdbec3d9e3f6))

- Unify token display using format_token_summary in TuiRenderer ([8235bd7](https://github.com/lanegrid/agtrace/commit/8235bd7c99b204ff312141dd0371ff03cd6ca7b1))

- Implement 4-layer view architecture with display models and views ([cbc72e7](https://github.com/lanegrid/agtrace/commit/cbc72e7c8c570fe31b3135c2d47b16b4041768c0))

- Move session display modules from output/ to views/session/ ([c7f3f70](https://github.com/lanegrid/agtrace/commit/c7f3f70ed5f74ad749cb27e41e4303ebffa81b5f))

- Move pack and doctor display modules from output/ to views/ ([0f6b70d](https://github.com/lanegrid/agtrace/commit/0f6b70d2d1646a907f98b1087d68454aabde9c3b))

- Migrate doctor_check.rs display logic to views/ and update PROGRESS.md ([c029b52](https://github.com/lanegrid/agtrace/commit/c029b524784fd9ea3f1abbb4f30b22ff45dc33b4))

- Migrate init.rs display logic to views/ and update PROGRESS.md ([34ec5ad](https://github.com/lanegrid/agtrace/commit/34ec5adcf847fc555c24846ceb62ece5d745dedd))

- Remove output/ directory and complete view architecture migration ([fed7e3f](https://github.com/lanegrid/agtrace/commit/fed7e3fcd136565ca1d3ebaa0e5d1da828e0b6d3))

- Add ui layer in agtrace-cli ([b9db92e](https://github.com/lanegrid/agtrace/commit/b9db92e91cb50877ffbaa7a31839570c457b6487))

- Remove unused public exports from engine analysis module ([881ef7b](https://github.com/lanegrid/agtrace/commit/881ef7b4e83453c40159c11c8795a0b1590fcd74))

- Extract runtime orchestration layer into agtrace-runtime crate ([f639aad](https://github.com/lanegrid/agtrace/commit/f639aad79fd708e4d59d12c552036fa2f41daa60))

- Move state management and reactor orchestration from CLI to runtime ([54b6829](https://github.com/lanegrid/agtrace/commit/54b682956899b6e44567089f1324236d4bd079a0))

- Remove intervention pipeline and simplify reactions to Continue/Warn only ([9b7c372](https://github.com/lanegrid/agtrace/commit/9b7c3726e242494684f9dba78ea4ad3d3596340c))

- Remove v2 directory from agtrace-providers and move modules to natural locations ([e762a2c](https://github.com/lanegrid/agtrace/commit/e762a2c38e8549476553d1b1e676a8a8a1cd685b))

- Rename agtrace-types v2 module to models and update all references ([7cd7739](https://github.com/lanegrid/agtrace/commit/7cd7739edffb1a14f7ad41ec0928010a5578b269))

- Remove v2 suffix from function names and comments ([09c9953](https://github.com/lanegrid/agtrace/commit/09c9953c8459ea67a5e6bf20b1ab17cbc4cbda91))

- Decouple engine from providers by using AgentEvent fixtures in tests ([9dc6925](https://github.com/lanegrid/agtrace/commit/9dc6925917b52271bb5c62c4322c598ecee4aaa3))

- Remove deprecated create_event method ([89f4131](https://github.com/lanegrid/agtrace/commit/89f41316c64033f9520ad31e46f92f283301b18b))

- Remove UUID redaction from agtrace-engine tests ([8f24172](https://github.com/lanegrid/agtrace/commit/8f241723d6bea75093484eb1c269671cc30a6fc0))

- Reduce handle_fs_event parameters using state and context structs ([16b76f4](https://github.com/lanegrid/agtrace/commit/16b76f47e791f8cfb265b37bec4db5fdc6600af7))

- Reorganize AGENTS.md and CLAUDE.md with .agent/skills structure and symlinks ([931b4d1](https://github.com/lanegrid/agtrace/commit/931b4d120366142de58f7caeed9f680718e3cdb0))

- Reorganize skills structure with .claude/skills as symlink to .agent/skills ([e07aa8c](https://github.com/lanegrid/agtrace/commit/e07aa8cae350536d110cb17ce6fdb38c16b9e051))

- Remove buffer size limit, rely on terminal height only ([098f023](https://github.com/lanegrid/agtrace/commit/098f023a786b3d038f242894ef20bce20af9d04c))

- Use effective context window limit for display (accounting for compaction buffer) ([e831f1e](https://github.com/lanegrid/agtrace/commit/e831f1ed5132addbf4908df8976f2a567653ce46))

- Reorganize agtrace-engine session files into session/ directory ([996112d](https://github.com/lanegrid/agtrace/commit/996112d024b8c4f6383b18003de8076c7499916f))

- Split assembler.rs into modular files with unit tests ([75512d5](https://github.com/lanegrid/agtrace/commit/75512d5e6db56de0a61a8686c387773d4fcdebe0))

- Refactor tool_mapping to use structured registry with ToolSpec ([cb97b0e](https://github.com/lanegrid/agtrace/commit/cb97b0eced7dc9c829e5d4ac8139d5ceedd8d354))

- Simplify watch - remove File mode, unify SessionUpdate, drop unused traits ([494b14b](https://github.com/lanegrid/agtrace/commit/494b14b6bd68c84e83e06c095978e7d34021e989))

- Remove unnecessary re-export layers (reactor, streaming) ([d948e23](https://github.com/lanegrid/agtrace/commit/d948e239bd7d605fc950a4524d2a4e04a23ec23e))

- Remove unused TuiRenderer reactor ([84b3c3c](https://github.com/lanegrid/agtrace/commit/84b3c3cb4d8c12dd3a3d18a8625d996b2fcaf903))

- Remove provider schema command and ProviderSchemaDisplay ([821c0ac](https://github.com/lanegrid/agtrace/commit/821c0ac6cefcaefffd5b1c04ae1d6f1ea395a6a0))

- Rename StreamEvent to WatchEvent ([76d0109](https://github.com/lanegrid/agtrace/commit/76d01090e9809c29c9879d5157f0eab572c6c89d))

- Remove non-essential reactors, focus on core monitoring ([a3872ff](https://github.com/lanegrid/agtrace/commit/a3872ff32ec33cb4ca35cb21ebb50a4ccff6c9e7))

- Unify show/watch display with pure rendering functions ([a6e218f](https://github.com/lanegrid/agtrace/commit/a6e218f22978c79dc1988bda0258f770002f5803))

- Unify timeline display to use shared format_event_with_start function ([7b502ee](https://github.com/lanegrid/agtrace/commit/7b502ee6a21c0b6129e47efdc93e93d6d5422975))

- Show token usage only at the end in show command, use shared component ([361fb0d](https://github.com/lanegrid/agtrace/commit/361fb0d95e342d8f78e8c441e9847a2b7e885882))

- Move token_usage_monitor and token_limits to agtrace-runtime ([250dd68](https://github.com/lanegrid/agtrace/commit/250dd68edf07b24179c519d0c81106bfaf9fc6c2))

- Move SessionLoader to agtrace-runtime as SessionRepository ([87b33aa](https://github.com/lanegrid/agtrace/commit/87b33aa4efb0a2b7e704aee73b372c2abe88aed9))

- Reorganize display_model, ui, views into presentation module with formatters, renderers, models structure ([b87aae0](https://github.com/lanegrid/agtrace/commit/b87aae0bfd69423788cd32489a02d94573ee2ae2))

- Finalize presentation module refactoring by updating all imports and removing backwards compatibility re-exports ([f3bf8ea](https://github.com/lanegrid/agtrace/commit/f3bf8ea76158044b882448ba71d30674fcf93d7d))

- Formatters now accept domain models directly instead of intermediate display models ([9ac841c](https://github.com/lanegrid/agtrace/commit/9ac841c3d7aca5c5b8f8f5cda30f5544673e2298))

- Finalize session formatter refactoring by removing unused models/session.rs ([21f52c9](https://github.com/lanegrid/agtrace/commit/21f52c9474143104861358db6b5acdfb68057003))

- Remove dead_code from presentation/formatters/session ([dbd3502](https://github.com/lanegrid/agtrace/commit/dbd350277e5c68db2c6974a6462c619bdbbb77c5))

- Doctor formatter now accepts domain data directly, removed intermediate DoctorCheckDisplay model ([516792c](https://github.com/lanegrid/agtrace/commit/516792cf16fe37a9b6cd493376e0590e397c1b2c))

- Init formatter now passes specific data directly, moved step result types to formatters, removed models directory ([ec0aa97](https://github.com/lanegrid/agtrace/commit/ec0aa977ee387a8bf71fe86311fb8844d5f3060c))

- Migrate formatters to View Struct pattern using std::fmt::Display trait ([21132b7](https://github.com/lanegrid/agtrace/commit/21132b74afbebfb3a45ded049f44de6fc402d644))

- Move token calculation logic from renderers to TokenUsageView::from_state ([83c39e3](https://github.com/lanegrid/agtrace/commit/83c39e3240db7e5a8bdb3f2d4bf59c4d8ea5a19c))

- Extract text and time utilities from renderers to formatters (Phase 1-2) ([9b00916](https://github.com/lanegrid/agtrace/commit/9b0091686d81eca227062a34861458dcc7677171))

- Extract session list formatting to formatters/session_list.rs (Phase 3) ([703acee](https://github.com/lanegrid/agtrace/commit/703acee985709ae4075b72cc13476e84c72b17f3))

- Extract path and tool formatters, update refresh.rs (Phases 4-7) ([24518b1](https://github.com/lanegrid/agtrace/commit/24518b1fe367edb820f73b6640dd0aac82bcdcc4))

- Eliminate duplicate code and improve architecture consistency across presentation layer ([9928d8c](https://github.com/lanegrid/agtrace/commit/9928d8ca505b0e0b763565bf3547fd4d45761577))

- *(cli)* Make compact default for session show, add --verbose for timeline view ([9c5c62e](https://github.com/lanegrid/agtrace/commit/9c5c62ede9ebe9792dce39cc7650ef9eabab0b91))

- *(cli)* Enforce presentation layer separation with ViewModels, presenters, and views ([0668d9c](https://github.com/lanegrid/agtrace/commit/0668d9c8927318359539af3c8297cd6bc6e23fc7))

- Remove backward compatibility re-exports, update all callers to use correct layers ([d2dbb2b](https://github.com/lanegrid/agtrace/commit/d2dbb2b3ef0c69da7400319a7d21a50aad849393))

- *(cli)* Remove deprecated renderers/models re-export, import from view_models directly ([56e4b49](https://github.com/lanegrid/agtrace/commit/56e4b499f59aafdaa07875033b76a6c9886f9221))

- *(cli)* Enforce presentation layer separation with ViewModels for SessionSummary, Reaction, SessionState ([0b2c926](https://github.com/lanegrid/agtrace/commit/0b2c9262fee6b78f2006c4c0f3627dd7efbadf43))

- *(formatters)* Remove domain dependencies to enforce pure utility layer ([f885b59](https://github.com/lanegrid/agtrace/commit/f885b5927ea3e8e6720ffc032b6d49758251bede))

- *(presentation)* Move TokenLimits computation from renderers to presenter layer ([013f808](https://github.com/lanegrid/agtrace/commit/013f808fe79fecaf7e65d3f74491bd8f98da2d53))

- Enforce layer boundaries by moving IO to services, presentation logic to view models, and removing cross-layer dependencies ([c084557](https://github.com/lanegrid/agtrace/commit/c08455721231ca3d6f0b4047b829419cc0862617))

- *(presentation)* Introduce shared/ module for cross-cutting concerns, satisfy both layer guard scripts with pragmatic layering approach ([033655f](https://github.com/lanegrid/agtrace/commit/033655f49bb32775b60f2174042b2e15b5d00575))

- *(init)* Move config and init logic to agtrace-runtime services ([192d46f](https://github.com/lanegrid/agtrace/commit/192d46f810022b78c8595d4ae6995112fb131ff1))

- *(init)* Move orchestration logic to runtime, follow watch pattern ([2bbd709](https://github.com/lanegrid/agtrace/commit/2bbd7094f2fd0d284979314a1110a9d19c41a6a5))

- *(init)* Separate progress and data, eliminate event stream complexity ([e05c8ee](https://github.com/lanegrid/agtrace/commit/e05c8ee1bf9e765bb4d96e29a0cc39844934dfdd))

- *(init)* Make init guidance-focused, return count instead of full session list ([989ada1](https://github.com/lanegrid/agtrace/commit/989ada1f5a419bd6da70a1c5140575ed8cced697))

- Extract business logic to runtime services (IndexService, DoctorService) ([e5f64bd](https://github.com/lanegrid/agtrace/commit/e5f64bde428ffda9405b532a71d7d177026aa23b))

- *(session)* Extract business logic to SessionService ([80b7ccc](https://github.com/lanegrid/agtrace/commit/80b7ccc74d029a01150e79ca352f99acaf475f29))

- *(services)* Extract business logic from handlers to runtime services ([55585ff](https://github.com/lanegrid/agtrace/commit/55585ffc7b807636d43accd05342b344719cee0c))

- *(session)* Centralize data access through SessionService ([989df79](https://github.com/lanegrid/agtrace/commit/989df795897b6c7d9feb3617d2d21ce564c85824))

- *(watch)* Extract business logic to runtime services and presenters ([fb0a9d7](https://github.com/lanegrid/agtrace/commit/fb0a9d768a322c3d1887ec16fdcabf66a99c859c))

- *(cli)* Remove --refresh option and related code branches ([f3863cd](https://github.com/lanegrid/agtrace/commit/f3863cd984ddd44a74c5e59c02e2858b3a6bc4d1))

- *(tui)* Phase 1 - replace raw crossterm with Ratatui widgets ([9858976](https://github.com/lanegrid/agtrace/commit/9858976ea7507afc3725eedc680e7287332aef63))

- *(tui)* Phase 2+3 - add event loop and move WatchService to background thread ([3f4adb2](https://github.com/lanegrid/agtrace/commit/3f4adb23a0590af09a5b9c118a45cd41d40c31c9))

- *(tui)* Restructure into component-based architecture with state management ([c7ccc76](https://github.com/lanegrid/agtrace/commit/c7ccc769ab32a7d1aa660eebdaef5eb8b45e4c8e))

- *(tui)* Implement pre-rendering strategy with mapper layer ([0b9133a](https://github.com/lanegrid/agtrace/commit/0b9133a11e9c099702042518cfc05468321ab9d6))

- *(runtime)* Restructure into 3-layer architecture (domain/storage/ops/runtime) ([15e364b](https://github.com/lanegrid/agtrace/commit/15e364bf1a08b237c85a8aaf20589cf90c9c2cbd))

- *(cli)* Migrate to workspace-oriented client interface ([5482fb0](https://github.com/lanegrid/agtrace/commit/5482fb0d6c9174e23e6a160fba4df14c1bf79033))

- *(runtime)* Remove legacy exports and migrate CLI to AgTrace facade ([42dafd3](https://github.com/lanegrid/agtrace/commit/42dafd38c0c56927415944c772a025bc34a1acd1))

- *(cli)* Consolidate types.rs into args.rs and remove config.rs, token_usage.rs ([663a191](https://github.com/lanegrid/agtrace/commit/663a1919afb8a378a7be5fc2350df45f09bea2d7))

- *(cli)* Remove unused Config/ProviderConfig re-exports ([50a8cd0](https://github.com/lanegrid/agtrace/commit/50a8cd08dc1402dc91ab068688b3bcadb2466115))

- *(cli)* Move ExecutionContext to handlers/context.rs ([596da03](https://github.com/lanegrid/agtrace/commit/596da030528ba4c98ad31d7d8cb2ac02f432217e))

- *(cli)* Remove ExecutionContext and use AgTrace directly in handlers ([6bf856c](https://github.com/lanegrid/agtrace/commit/6bf856ca8fde29ce55d9346b345bcc1759c9c2e7))

- *(cli)* Show help instead of guidance when no command provided ([8a0143d](https://github.com/lanegrid/agtrace/commit/8a0143df28c0357695cdaa5b6c304a8d8e0b26af))

- *(cli)* Make watch command TUI-only, remove non-TTY fallback ([23e6f96](https://github.com/lanegrid/agtrace/commit/23e6f96783d47283de795089bcdb3f6efc62d5ef))

- *(cli)* Move TUI view creation to commands.rs for consistency ([51f726f](https://github.com/lanegrid/agtrace/commit/51f726fc0a58638601f607eef02f3bf4ebb37546))

- *(runtime)* Replace Runtime/Reactor with SessionStreamer/WorkspaceSupervisor ([f1df22f](https://github.com/lanegrid/agtrace/commit/f1df22fa2e2708ace839df96f1fdb516142f606f))

- *(runtime)* Make AgTrace thread-safe by using PathBuf instead of Arc<Database> ([27a3ead](https://github.com/lanegrid/agtrace/commit/27a3ead8879ccc831c3bf894b5ca633f4e069c7c))

- *(runtime)* Consolidate streamer initialization and improve file state management ([e63c056](https://github.com/lanegrid/agtrace/commit/e63c056ae705631103ba7b5f843ea20d0c739787))

- *(tui)* Remove footer and redesign context box with compact layout ([68d1754](https://github.com/lanegrid/agtrace/commit/68d175456ee969c5b83aa7fca41053a4d0ddb313))

- *(tui)* Restructure watch view to 3-section layout (Session Header / Global Life Gauge / Consumption History) ([1adfaae](https://github.com/lanegrid/agtrace/commit/1adfaaee8e57baf0624cfe8073b1c4e4edf525ca))

- *(tui)* Unify SessionHeader and GlobalLifeGauge into single DashboardComponent per requirements ([8abb2ef](https://github.com/lanegrid/agtrace/commit/8abb2ef68d51f858a9c4d1a6daa743778b20ee41))

- *(runtime)* StreamContext now maintains AgentSession and sends it with events ([9f737b2](https://github.com/lanegrid/agtrace/commit/9f737b2dfb191b7a3c20685d0befefebf7672582))

- *(engine,cli)* Move TUI presentation logic to engine for robustness ([abf9157](https://github.com/lanegrid/agtrace/commit/abf915731d1fee78aa04b914be73a68bfed930f2))

- Move ToolCallPayload::from_raw to providers layer for proper separation of concerns ([fedc146](https://github.com/lanegrid/agtrace/commit/fedc14600dbc0df97951d439b306de93f540fa2b))

- *(providers)* Move tool normalization from shared util to provider-specific functions ([e2a423b](https://github.com/lanegrid/agtrace/commit/e2a423b6cac938238512b8c8e3b8980c23193372))

- *(providers)* Separate concerns into LogProvider, SessionParser, and ToolMapper traits ([c3a6cb2](https://github.com/lanegrid/agtrace/commit/c3a6cb2fdd06b5c97848edb61a1351f8e0f30733))

- *(providers)* Rename LogProvider to LogDiscovery and implement Facade pattern ([b1951e1](https://github.com/lanegrid/agtrace/commit/b1951e1f79e7df6b26a4333798eab231b7c1e8f0))

- *(providers)* Complete Facade pattern with scan_legacy bridge method ([02e1cf7](https://github.com/lanegrid/agtrace/commit/02e1cf7141ad3daf38128484e3bdd0f2ca46195f))

- *(providers)* Split normalize.rs into parser.rs and mapper.rs for all providers ([8dd7d21](https://github.com/lanegrid/agtrace/commit/8dd7d2162b43353b28b956079cc83ff772f78660))

- *(consumers)* Migrate all LogProvider consumers to ProviderAdapter traits ([e1a1508](https://github.com/lanegrid/agtrace/commit/e1a15083cdf4b1f46e50889d250f6ab01819f131))

- *(providers)* Remove LogProvider trait and legacy facade implementations ([d38ff13](https://github.com/lanegrid/agtrace/commit/d38ff13e0cf492dc62d4845316245ed6e5962fc5))

- *(runtime)* Share Database instance via Arc<Mutex> across all operations ([435b630](https://github.com/lanegrid/agtrace/commit/435b630088e5e8795a0d2f57e7127daef54dc250))

- *(runtime)* Implement read-through indexing in SessionOps ([424474a](https://github.com/lanegrid/agtrace/commit/424474aef2bfb02bd95e056394e6292d006dd9db))

- *(runtime)* Add read-through indexing to find and pack_context methods ([1759f3d](https://github.com/lanegrid/agtrace/commit/1759f3d0ba98d3e5d153bb5b4431278d7167cb0a))

- *(runtime)* Apply read-through indexing to all Ops (ProjectOps, InsightOps) ([1e829cf](https://github.com/lanegrid/agtrace/commit/1e829cfa5e48d2335a532869c416bced067c2e2f))

- *(cli)* Implement unified v2 presentation architecture with project list migration ([8cc96e1](https://github.com/lanegrid/agtrace/commit/8cc96e1da2dd1b2632c18dbde0f49219beea30c4))

- *(cli)* Migrate session list to v2 presentation with smart guidance and filter summary ([a3a7229](https://github.com/lanegrid/agtrace/commit/a3a722987364d9bd934ba4945337409c1cd2e57a))

- *(cli)* Migrate provider commands to v2 with smart guidance for config workflow ([d95634a](https://github.com/lanegrid/agtrace/commit/d95634ac04a99f1aa68556d74c69b70ed712423c))

- *(cli)* Migrate index commands to v2 with progress events and smart rebuild suggestions ([51afa70](https://github.com/lanegrid/agtrace/commit/51afa702afd4d72f454e6d60b1e9278089afe38a))

- *(cli)* Migrate doctor commands to v2 and update tests for v2 JSON format ([3864fca](https://github.com/lanegrid/agtrace/commit/3864fca9c4aa505eb55adb38eb9cb9c3c66c92ae))

- *(cli)* Replace ConsolePresentable with Display trait for testability ([594986d](https://github.com/lanegrid/agtrace/commit/594986dcb5ca1634bf0e9109e30b683877b6dcd2))

- *(cli)* Migrate session show to v2 with TUI-centric context analysis and turn metrics ([e008185](https://github.com/lanegrid/agtrace/commit/e00818578f7320b7cd11de60ff54263a7fb9caab))

- *(cli)* Move all display logic from presenter to Display impl ([4acf430](https://github.com/lanegrid/agtrace/commit/4acf430e3bd108ddd17b10db555aeeab56c40e74))

- *(cli)* Separate presentation concerns by moving formatting to view model Display ([fdb64b9](https://github.com/lanegrid/agtrace/commit/fdb64b90bfa00538c0c7a5165f68d938590829e2))

- *(cli)* Separate data transformation and guidance logic in session presenter ([69fd11a](https://github.com/lanegrid/agtrace/commit/69fd11a0504bce157c14c96cc6238b6a32a6c2a8))

- *(cli)* Separate View from ViewModel by extracting rendering logic to v2/views layer ([e27f17f](https://github.com/lanegrid/agtrace/commit/e27f17ffcacf1a8d9359644e058d8be3d3a94c83))

- *(cli)* Extract all ViewModels rendering logic to dedicated views layer ([6c7cfc2](https://github.com/lanegrid/agtrace/commit/6c7cfc298791a75505943d0155e23774b381c5ce))

- *(cli)* Unify ViewMode support across all commands with ViewModeArgs ([ad3d9dc](https://github.com/lanegrid/agtrace/commit/ad3d9dc9e8885f8cda561707ad75c3d9742a8713))

- *(cli)* Reorganize presentation layer into flat v1/v2 structure for clear separation ([30aedf1](https://github.com/lanegrid/agtrace/commit/30aedf1a5091b0dd74ad8db91fae4a73701d3e2b))

- *(cli)* Migrate lab export handler to v2 presentation layer ([4983344](https://github.com/lanegrid/agtrace/commit/498334453a0190355a7e80ff201e9bbf0726c8d5))

- *(cli)* Migrate pack handler to v2 presentation layer ([1a225aa](https://github.com/lanegrid/agtrace/commit/1a225aa249697aed9d524bb0645f8876fb2c1e1a))

- *(cli)* Migrate init handler to v2 presentation layer ([2b1cb8b](https://github.com/lanegrid/agtrace/commit/2b1cb8b2165adb062ff36d01d231feb95dd7cc5d))

- *(cli)* Migrate lab stats handler to v2 presentation layer ([ec0ab46](https://github.com/lanegrid/agtrace/commit/ec0ab46a2af3b0a04d4ad6e9b8f54b22873125f1))

- *(cli)* Move IndexEvent from v1 to v2 presentation layer ([11d8d3b](https://github.com/lanegrid/agtrace/commit/11d8d3b1d3e49511c19eb573ee7164a7193f8279))

- *(cli)* Migrate lab grep handler to v2 presentation layer ([d522307](https://github.com/lanegrid/agtrace/commit/d522307f85d23200d7e8eb75692d1994cf3cef12))

- *(cli)* Migrate watch handler to v2 presentation layer ([2a4bb6f](https://github.com/lanegrid/agtrace/commit/2a4bb6fb6dbd50df87058450403d4dc2354a2e8c))

- *(cli)* Remove unused v1 presenters and event view models ([f1e2257](https://github.com/lanegrid/agtrace/commit/f1e2257efe61e7d216f297f2410fe5f9bf94f0c4))

- *(cli)* Add v2 watch presentation layer (view models and presenters) ([9bbd3dc](https://github.com/lanegrid/agtrace/commit/9bbd3dc4efb14d6ca76bd1af34ac853020d4b511))

- *(cli)* Fix clippy warnings in watch view (redundant closure and explicit counter) ([b2825ac](https://github.com/lanegrid/agtrace/commit/b2825ac8798df5c98b64fedcc1ab720a83285191))

- *(cli)* Extract type aliases for complex nested types in lab presenter ([845e459](https://github.com/lanegrid/agtrace/commit/845e45924a5c655958f503f17e66369e4ec061d7))

- *(cli)* Introduce Component pattern for TUI v2 to prevent Renderer becoming Big Ball of Mud ([1448394](https://github.com/lanegrid/agtrace/commit/14483940ea281237a96ff9bc675dcd60945b2b86))

- *(cli)* Remove unused v1 presentation code, keeping only watch-related components ([eb41ed3](https://github.com/lanegrid/agtrace/commit/eb41ed3e338a6708af4b5490016913f0a9e72bf3))

- *(cli)* Migrate watch --mode tui to v2 architecture, remove presentation/v1 ([da1001a](https://github.com/lanegrid/agtrace/commit/da1001ae208ef9a6d27faf6428a55b0edad8fc21))

- *(cli)* Remove v2 namespace, make presentation layer default ([cb94fba](https://github.com/lanegrid/agtrace/commit/cb94fbaaac491098cd45c53e2c5b1918fd15d3c4))

- *(cli)* Remove all v2 suffixes and references from function names and comments ([250faf5](https://github.com/lanegrid/agtrace/commit/250faf59500d4de825c54f824fe2dcc79b9d1c86))

- *(cli)* Modularize args, simplify commands dispatcher, and standardize handler patterns ([51d4fb3](https://github.com/lanegrid/agtrace/commit/51d4fb316f70cdb9008b439d6aafa081b672c968))

- *(cli)* Improve session views with testable constants and unit tests ([83b6803](https://github.com/lanegrid/agtrace/commit/83b68031591b8ac154dad4f2f1c554f7393ba524))

- *(providers)* Modularize provider structure by extracting discovery and legacy types ([12a4ac3](https://github.com/lanegrid/agtrace/commit/12a4ac34b34abc2bdf892911afd438fed1ff208c))

- Adopt workspace inheritance for Cargo.toml files ([df04850](https://github.com/lanegrid/agtrace/commit/df048500fe0a71d61bd69a1fc5dde0e46586203c))

- Use npm OIDC trusted publishing instead of token-based auth while keeping cargo-dist for builds ([e2f2352](https://github.com/lanegrid/agtrace/commit/e2f235279cb8c7f9bb815b6abf0dd40eb2bdc8b7))


### Styling

- Run cargo fmt ([48043ab](https://github.com/lanegrid/agtrace/commit/48043ab986e0022c16a0752db20f75ba63619b8c))


### Testing

- Add snapshot tests for Codex and Claude providers ([891bbf2](https://github.com/lanegrid/agtrace/commit/891bbf2c805c66d6a6e5c6272cc2d38b9ae646ea))

- Remove unnecessary tests and fixtures ([8db9339](https://github.com/lanegrid/agtrace/commit/8db93391290820b9adf31ddb70e25c934b2eea6d))

- Change snapshot format from pretty JSON to compact JSONL ([c52093d](https://github.com/lanegrid/agtrace/commit/c52093d45f86cbffaadacde67b34624f4573bf65))

- Update snapshots for schema compatibility changes ([fd76f25](https://github.com/lanegrid/agtrace/commit/fd76f25f0d6a6ffa673c822d36bbbdf84cb2ad97))

- Update snapshots for tool name normalization and duplicate removal ([4e6b289](https://github.com/lanegrid/agtrace/commit/4e6b2892d7f1fec290908c42c8296659e6e25895))

- Remove obsolete backwards compatibility tests ([f34acf9](https://github.com/lanegrid/agtrace/commit/f34acf94f131120c20fb2965f2aa297a713fdc0b))

- Update engine activity snapshots after refactoring ([8899cb2](https://github.com/lanegrid/agtrace/commit/8899cb243905dad9254e0017bac6dd87b9f0f967))

- Improve snapshot readability with pretty JSON format ([d2df808](https://github.com/lanegrid/agtrace/commit/d2df80836cdb87a4a9718242bdb2b920a9f2e487))

- Add CLI help snapshot tests for all commands ([f656f38](https://github.com/lanegrid/agtrace/commit/f656f38fd30e7ca68877af7d1ed4e79034ee609a))

- Update help snapshots and finalize Phase 1-4 ([59237d1](https://github.com/lanegrid/agtrace/commit/59237d1174109153e24bc2b173ebf381c4a17ff3))

- Add v2 provider snapshot tests with UUID redaction ([6caf838](https://github.com/lanegrid/agtrace/commit/6caf8383bf20b8e3b3b9844eea6463ca7957e3f6))

- Remove redundant test_discover_project_root_with_env_var ([e4df870](https://github.com/lanegrid/agtrace/commit/e4df87058f4d65de5849c8fa0b55b15eacaac910))

- Add reactor unit tests and documentation ([1394841](https://github.com/lanegrid/agtrace/commit/13948411297a7e9c139ce44bc23a092b258f5055))

- Add integration tests for init, index, and session show workflows ([b01eecc](https://github.com/lanegrid/agtrace/commit/b01eecc1bd23d6e3ec3df398a1a8948a4ffc81f1))

- Refactor integration tests into modular files (<200 lines each) ([21edcb1](https://github.com/lanegrid/agtrace/commit/21edcb14b8d46f58de26a3b9678d70526c3c949f))

- Update snapshots for cache_creation_input_tokens field ([7904270](https://github.com/lanegrid/agtrace/commit/7904270bba26f52f241f355f4ef4022813ebeddc))

- Remove orphaned non-v2 snapshot files ([fce05e7](https://github.com/lanegrid/agtrace/commit/fce05e7bcbe233fef42c1f522d8e8f4aeab16927))

- Rename span_v2_snapshots.rs to span_snapshots.rs ([5511854](https://github.com/lanegrid/agtrace/commit/55118546ca4069e14d7fd4f2a34019dab0895437))

- Remove v2 suffix from test function and snapshot names ([b7c144f](https://github.com/lanegrid/agtrace/commit/b7c144f958cd22baeb74f735ebd37aa965f5ee4b))

- Rename v2 snapshot files to standard names ([ba70917](https://github.com/lanegrid/agtrace/commit/ba709175ebc4b717fd6df2067136e1a34c24c5a9))

- Remove session rotation test and add TODO for future implementation ([1669b98](https://github.com/lanegrid/agtrace/commit/1669b98a6cc8e85a8a17556855be0b5ca9e4a53c))

- *(types)* Add comprehensive tests for ToolCallPayload normalization and fix breaking changes ([e41c47b](https://github.com/lanegrid/agtrace/commit/e41c47baae31c6219e2b4b1952830eb669a4695b))

- *(cli)* Update session tests to validate v2 JSON structure instead of removed filtering flags ([6fb898e](https://github.com/lanegrid/agtrace/commit/6fb898eddc9e29ca3e48ed4b63bbc94c79c99ef6))


### Debug

- Add detailed logging for watch command session filtering and file events ([d37ba50](https://github.com/lanegrid/agtrace/commit/d37ba507a5c8ee675b74f790371deaf8762666bc))


### Deps

- Upgrade to latest versions (notify 8.2, rusqlite 0.38, crossterm 0.29, toml 0.9, dirs 6.0) ([1dc4697](https://github.com/lanegrid/agtrace/commit/1dc46979368b27a581935badfc98ed39c9e00f09))


### Improve

- Enhance refresh UI visibility and tool display ([84b9650](https://github.com/lanegrid/agtrace/commit/84b9650bb4a4600e72e7d1a83270d3a3b038df10))

- Show project info in header and use relative paths for file display ([f7cd6a5](https://github.com/lanegrid/agtrace/commit/f7cd6a5f33ea6b2047d475add7a4443d52f21d0a))


### Remove

- Delete deprecated Span API in favor of AgentSession ([8f4b147](https://github.com/lanegrid/agtrace/commit/8f4b147db3db3883b426140f180d1df581150e03))


