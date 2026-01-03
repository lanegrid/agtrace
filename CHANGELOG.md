# Changelog

All notable changes to this project will be documented in this file.

## [0.5.0] - 2026-01-03

### Bug Fixes

- Session show displays correct ID, provider, and project info ([c72835b](https://github.com/lanegrid/agtrace/commit/c72835b2791098a962bc1f1e04ed805095001a79))

- Normalize turn snippets to single line in compact view ([5667977](https://github.com/lanegrid/agtrace/commit/56679774946434db89b865842ee8dece10ddcd48))

- Normalize text truncation in user-facing views to handle newlines ([f37a2f8](https://github.com/lanegrid/agtrace/commit/f37a2f8f128dc39ce1b01c15f0d70bee6e1f7450))

- Change default ViewMode to Compact and merge Standard into Verbose (#34) ([5c2d6e9](https://github.com/lanegrid/agtrace/commit/5c2d6e9356329954fb69a472f3878057b958c0bd))

- Normalize all session header fields to prevent newline display issues ([e87960b](https://github.com/lanegrid/agtrace/commit/e87960b6471b382ad112e260835c09f2829328f4))

- Tune MCP get_turns defaults to data-driven limits (3k chars, 30 steps) for safe token usage ([d47d295](https://github.com/lanegrid/agtrace/commit/d47d295cad45e85213f909ce46551c2498a6147b))


### Documentation

- Add real-world MCP workflow example and fix broken links ([bcc5b9a](https://github.com/lanegrid/agtrace/commit/bcc5b9a069c959cd477c42787d858f4722e15c85))

- Update MCP tool references to Random Access APIs and remove unused legacy types ([f2b04a7](https://github.com/lanegrid/agtrace/commit/f2b04a7b9df598d82f0aa450f39af40d102605a9))

- Update MCP API references in comments and help text to Random Access APIs ([d176a0c](https://github.com/lanegrid/agtrace/commit/d176a0c774bb5bebba32032c024db1a46e7cad64))

- Reposition agtrace as memory system for AI agents via MCP ([ffb68ce](https://github.com/lanegrid/agtrace/commit/ffb68ce32033f17e7754348a81fba822709caabc))


### Features

- Change --project-root to --project with --project-root as alias ([346ecda](https://github.com/lanegrid/agtrace/commit/346ecdaae4b095b01f531eaeeee8e34ea663c3ff))

- Display both project_hash and project_root in session show ([40b1e8d](https://github.com/lanegrid/agtrace/commit/40b1e8ddfb28a4eaba32948484827520c3b27ed7))

- Improve session show display with vertical layout and smart path formatting ([f6a4a32](https://github.com/lanegrid/agtrace/commit/f6a4a320322a77f16ad08a43ecc914d1f50dbfd0))

- *(mcp)* Add project filter to search_event_previews ([211d667](https://github.com/lanegrid/agtrace/commit/211d6678aa997ed4f4578f09764ba44a16363085))

- Limit project list to top 10 by session count ([11a49e5](https://github.com/lanegrid/agtrace/commit/11a49e5d822fa8135697d00d1cdf0d0b009a4c46))

- Improve compact session header with clear key-value format ([298e209](https://github.com/lanegrid/agtrace/commit/298e20990aa1a77a8ae33a18aa29e56fe7d4c7ce))

- Compact mode shows turn metadata and tool count only ([506a2cd](https://github.com/lanegrid/agtrace/commit/506a2cd69c79ec19edeada05319d244df01af972))

- Add project root column and aligned headers to session list ([6435acf](https://github.com/lanegrid/agtrace/commit/6435acf9af401ed903c0dce334beff9b5b221228))

- Auto-fix README version references in release script ([e8cd078](https://github.com/lanegrid/agtrace/commit/e8cd0786c44b5930c8f92898743519afd954497c))

- Add Random Access API service layer with safety valves ([197d61a](https://github.com/lanegrid/agtrace/commit/197d61a6fca01065e0bf72408d47030200bc3340))

- Update MCP test suite for Random Access APIs with UTF-8 safe truncation ([31196e0](https://github.com/lanegrid/agtrace/commit/31196e028c3f24b9a29e155992b4ef0dc913c277))

- Add SessionOrder parameter to session listing queries ([edc3544](https://github.com/lanegrid/agtrace/commit/edc3544cdbd90d83ba6674ebc083d2b4fd5b477d))


### Refactor

- Simplify compact mode to show turn metadata and user msg only ([a07ea3b](https://github.com/lanegrid/agtrace/commit/a07ea3b6e3e167a6950b2702b2bc60a0c3b27915))

- Compact mode displays one line per turn ([4af279e](https://github.com/lanegrid/agtrace/commit/4af279e0f80282cbc5cfb15dc485ac57dbd8aaee))

- Remove legacy MCP APIs, keep only Random Access APIs ([63968e5](https://github.com/lanegrid/agtrace/commit/63968e533168f32255a66cf6e71f26562210f190))


## [0.5.0] - 2026-01-03

### Bug Fixes

- Session show displays correct ID, provider, and project info ([c72835b](https://github.com/lanegrid/agtrace/commit/c72835b2791098a962bc1f1e04ed805095001a79))

- Normalize turn snippets to single line in compact view ([5667977](https://github.com/lanegrid/agtrace/commit/56679774946434db89b865842ee8dece10ddcd48))

- Normalize text truncation in user-facing views to handle newlines ([f37a2f8](https://github.com/lanegrid/agtrace/commit/f37a2f8f128dc39ce1b01c15f0d70bee6e1f7450))

- Change default ViewMode to Compact and merge Standard into Verbose (#34) ([5c2d6e9](https://github.com/lanegrid/agtrace/commit/5c2d6e9356329954fb69a472f3878057b958c0bd))

- Normalize all session header fields to prevent newline display issues ([e87960b](https://github.com/lanegrid/agtrace/commit/e87960b6471b382ad112e260835c09f2829328f4))

- Tune MCP get_turns defaults to data-driven limits (3k chars, 30 steps) for safe token usage ([d47d295](https://github.com/lanegrid/agtrace/commit/d47d295cad45e85213f909ce46551c2498a6147b))


### Documentation

- Add real-world MCP workflow example and fix broken links ([bcc5b9a](https://github.com/lanegrid/agtrace/commit/bcc5b9a069c959cd477c42787d858f4722e15c85))

- Update MCP tool references to Random Access APIs and remove unused legacy types ([f2b04a7](https://github.com/lanegrid/agtrace/commit/f2b04a7b9df598d82f0aa450f39af40d102605a9))

- Update MCP API references in comments and help text to Random Access APIs ([d176a0c](https://github.com/lanegrid/agtrace/commit/d176a0c774bb5bebba32032c024db1a46e7cad64))

- Reposition agtrace as memory system for AI agents via MCP ([ffb68ce](https://github.com/lanegrid/agtrace/commit/ffb68ce32033f17e7754348a81fba822709caabc))


### Features

- Change --project-root to --project with --project-root as alias ([346ecda](https://github.com/lanegrid/agtrace/commit/346ecdaae4b095b01f531eaeeee8e34ea663c3ff))

- Display both project_hash and project_root in session show ([40b1e8d](https://github.com/lanegrid/agtrace/commit/40b1e8ddfb28a4eaba32948484827520c3b27ed7))

- Improve session show display with vertical layout and smart path formatting ([f6a4a32](https://github.com/lanegrid/agtrace/commit/f6a4a320322a77f16ad08a43ecc914d1f50dbfd0))

- *(mcp)* Add project filter to search_event_previews ([211d667](https://github.com/lanegrid/agtrace/commit/211d6678aa997ed4f4578f09764ba44a16363085))

- Limit project list to top 10 by session count ([11a49e5](https://github.com/lanegrid/agtrace/commit/11a49e5d822fa8135697d00d1cdf0d0b009a4c46))

- Improve compact session header with clear key-value format ([298e209](https://github.com/lanegrid/agtrace/commit/298e20990aa1a77a8ae33a18aa29e56fe7d4c7ce))

- Compact mode shows turn metadata and tool count only ([506a2cd](https://github.com/lanegrid/agtrace/commit/506a2cd69c79ec19edeada05319d244df01af972))

- Add project root column and aligned headers to session list ([6435acf](https://github.com/lanegrid/agtrace/commit/6435acf9af401ed903c0dce334beff9b5b221228))

- Auto-fix README version references in release script ([e8cd078](https://github.com/lanegrid/agtrace/commit/e8cd0786c44b5930c8f92898743519afd954497c))

- Add Random Access API service layer with safety valves ([197d61a](https://github.com/lanegrid/agtrace/commit/197d61a6fca01065e0bf72408d47030200bc3340))

- Update MCP test suite for Random Access APIs with UTF-8 safe truncation ([31196e0](https://github.com/lanegrid/agtrace/commit/31196e028c3f24b9a29e155992b4ef0dc913c277))

- Add SessionOrder parameter to session listing queries ([edc3544](https://github.com/lanegrid/agtrace/commit/edc3544cdbd90d83ba6674ebc083d2b4fd5b477d))


### Refactor

- Simplify compact mode to show turn metadata and user msg only ([a07ea3b](https://github.com/lanegrid/agtrace/commit/a07ea3b6e3e167a6950b2702b2bc60a0c3b27915))

- Compact mode displays one line per turn ([4af279e](https://github.com/lanegrid/agtrace/commit/4af279e0f80282cbc5cfb15dc485ac57dbd8aaee))

- Remove legacy MCP APIs, keep only Random Access APIs ([63968e5](https://github.com/lanegrid/agtrace/commit/63968e533168f32255a66cf6e71f26562210f190))


## [0.4.2] - 2026-01-03

### Refactor

- Encapsulate sidechain filtering in SDK layer ([000cae9](https://github.com/lanegrid/agtrace/commit/000cae9f381660398ed52fe08683c78ed2246278))


## [0.4.1] - 2026-01-03

### Bug Fixes

- Unify top-level help message between -h and --help ([5631f15](https://github.com/lanegrid/agtrace/commit/5631f15e0b7aeea3aebdc7a1b7c347fa29c9692f))


### Documentation

- Add MCP troubleshooting for Node.js version managers ([0f99972](https://github.com/lanegrid/agtrace/commit/0f999728ef04e7b2cd6742de6ca4250c467906ce))

- Modernize MCP messaging and restructure README around watch and mcp values ([ef14681](https://github.com/lanegrid/agtrace/commit/ef146816220164b4e80b0104e836faa2214e732a))

- Fix Gemini CLI mcp add syntax (uses positional args, not -- separator) ([e458773](https://github.com/lanegrid/agtrace/commit/e4587732bac670a9228a6c44afde375744e8679c))

- Note Gemini CLI not yet supported (requires Content-Length framing) ([43a745d](https://github.com/lanegrid/agtrace/commit/43a745d130d9ac367201a01e382db72878a9710f))


### Refactor

- Update MCP tools from get_session_details/search_events to specialized tools ([31a7919](https://github.com/lanegrid/agtrace/commit/31a791934b2c52cdbf271b19fc6e5768004fc9bc))

- Reorder subcommands by importance and improve descriptions ([59ad2eb](https://github.com/lanegrid/agtrace/commit/59ad2eb31943ddd5c3ec679cd588d88a503c86a1))

- Hide less-used global options in -h to shorten help output ([8589bf0](https://github.com/lanegrid/agtrace/commit/8589bf09987c29edb26a19b1b1b7f85fdbdf0c86))


## [0.4.0] - 2026-01-02

### Bug Fixes

- Use numeric id instead of null in MCP error responses for Zod validation ([2014a57](https://github.com/lanegrid/agtrace/commit/2014a575a2c5a523f8d14b7b1d8232cc634750b2))

- Add cursor parameter to MCP tool schemas for list_sessions and search_events ([d3dfdb5](https://github.com/lanegrid/agtrace/commit/d3dfdb5b232d95053dee4ebce89e0491d579af53))

- Move provider filter from app layer to DB layer for accurate session filtering ([5d2ef3f](https://github.com/lanegrid/agtrace/commit/5d2ef3f4df1b36b15714176c579bbf9c41e097db))

- Resolve clippy warnings in MCP tests (use cargo_bin! macro, remove unused import) ([a390e7d](https://github.com/lanegrid/agtrace/commit/a390e7de39940b8f868fcd8dac8adcdb895d6fdd))

- Change SessionMetadata.session_id from Uuid to String for test compatibility ([8a73f01](https://github.com/lanegrid/agtrace/commit/8a73f0132a55f4573611de55f9af15fc850fc3a4))

- *(providers)* Truncate session snippets to 200 chars to prevent oversized MCP responses ([9273aa9](https://github.com/lanegrid/agtrace/commit/9273aa9689ada47d4773234613e27ef45d14d95f))

- *(mcp)* Truncate session summary and turns to prevent token limit errors ([9bf6fd9](https://github.com/lanegrid/agtrace/commit/9bf6fd9a067c2e850fc7580b1f5ab6d774893832))

- *(cli)* Rename lab export --format to --export-format to avoid conflict with global --format ([dc2e583](https://github.com/lanegrid/agtrace/commit/dc2e583645cf3f691154d9c5eea6249857bef10a))


### Documentation

- *(mcp)* Update tools interface for v0.4.0 with search_event_previews, get_event_details, structured errors, and migration guide ([821add1](https://github.com/lanegrid/agtrace/commit/821add115517c546e82ef189f2c1aacf0e85e677))

- *(mcp)* Update mcp-server.md for v0.4.0 tools and suppress deprecated search_events warning ([c3d8c7c](https://github.com/lanegrid/agtrace/commit/c3d8c7cb3f0f871acb9cf6553c6be8e5616bdbee))

- Replace XDG terminology with system data directory ([1e22a18](https://github.com/lanegrid/agtrace/commit/1e22a1836964d9375d1ad19f9c279845c7bd16cb))

- Fix command examples to match actual CLI implementation ([1c41c6e](https://github.com/lanegrid/agtrace/commit/1c41c6e3b26fc2a9b3c8f4aaa39f31d5a75a6ba8))

- Add MCP (Model Context Protocol) section for v0.4.0 ([a15fc90](https://github.com/lanegrid/agtrace/commit/a15fc900e5064dd4c554b1fdac450e9fbeb9c654))


### Features

- Add idempotent resume mode and --yes flag to prepare-release.sh ([f862e15](https://github.com/lanegrid/agtrace/commit/f862e153d2e655dc63b96662106f3377cc7128e3))

- Add MCP server for AI-native observability (issue #32) ([a288dff](https://github.com/lanegrid/agtrace/commit/a288dff780fc8391ec56f2dc1745e43e100105b5))

- Add dev test-mcp command to diagnose MCP response sizes and fix list_sessions default limit ([54f0eef](https://github.com/lanegrid/agtrace/commit/54f0eef0e9460ae68be7d3d453426d5c6c4bc183))

- Add comprehensive tests for all detail_level options in dev test-mcp command ([d55e877](https://github.com/lanegrid/agtrace/commit/d55e877546da38777ec8efc83ea30cc43be0aebf))

- Implement cursor-based pagination for MCP tools per specification ([79d9176](https://github.com/lanegrid/agtrace/commit/79d9176fc4006e65cc4e89123a380bd7b13bce5c))

- *(mcp)* Add foundational types for MCP tools redesign (Provider, EventType, McpError, McpResponse) ([9eb77db](https://github.com/lanegrid/agtrace/commit/9eb77db2b8d1f9dc83990fed005f2bdead22fad9))

- *(mcp)* Complete tools redesign - split search_events, add structured errors, standardize responses ([ad8449e](https://github.com/lanegrid/agtrace/commit/ad8449e2c89759edb8692d463ab75fd22916609d))

- *(mcp)* Improve error responses per MCP spec 2024-11-05 and add project_root parameter ([602fce3](https://github.com/lanegrid/agtrace/commit/602fce3c8a2f5b3066707c23254c4f94fbf4259b))

- *(mcp)* Add workflow and data source hints to tool descriptions ([ce74fd7](https://github.com/lanegrid/agtrace/commit/ce74fd79f4dbf0dbda1d3f3129d86d013ee8dde2))

- *(mcp)* Implement approach-b specialized session tools with response metadata ([b919a73](https://github.com/lanegrid/agtrace/commit/b919a73009080b4a395c443dfe8630ddaf6ef155))

- *(types)* Add SessionMetadata for clean separation of DB-derived and event-derived data ([324784b](https://github.com/lanegrid/agtrace/commit/324784b97953da31197748194114a44bcfb3626c))

- *(mcp)* Add tool-aware smart truncation for turn steps ([1965937](https://github.com/lanegrid/agtrace/commit/1965937f46daf04b606eb23e35f50f69f8192b72))

- *(mcp)* Add minimal context to tool summaries in get_session_turns ([e67cab9](https://github.com/lanegrid/agtrace/commit/e67cab9069139156d8ad794aba719408392b64e4))


### Miscellaneous Tasks

- Remove unnecessary docs ([7c7ff89](https://github.com/lanegrid/agtrace/commit/7c7ff897ffad19bc25ebb9143cd45c93cde43e7f))


### Refactor

- Add typed DTOs for MCP tools interface to improve type safety and maintainability ([f7a77fb](https://github.com/lanegrid/agtrace/commit/f7a77fb4a5a564fb8a221178a0a210f780b7a738))

- Restructure MCP DTOs with hierarchical detail levels and smart tool summarization ([a59c55c](https://github.com/lanegrid/agtrace/commit/a59c55cf1a47cffa6e762e0f8cabd24bda03c0d5))

- Use schemars to auto-generate JSON Schema from Rust types, eliminating schema drift ([d767cac](https://github.com/lanegrid/agtrace/commit/d767cacf79b148d7bead8a2346d711864ab2d5c0))

- *(mcp)* Remove docs, consolidate design rationale into code comments ([d68ad61](https://github.com/lanegrid/agtrace/commit/d68ad61c79dbe018f1d34b61760eaf555f78fd96))

- *(mcp)* Remove deprecated SearchEventsArgs and fix all lint warnings ([55ab644](https://github.com/lanegrid/agtrace/commit/55ab644da53e645735e4a4c8b0527e6af839aa2d))

- *(mcp)* Remove hint fields from MCP responses ([45e537f](https://github.com/lanegrid/agtrace/commit/45e537f463fdc46ffb89b63c4eba577759c474e3))

- *(mcp)* Rename get_session_details to get_session_by_id ([489fc28](https://github.com/lanegrid/agtrace/commit/489fc28c5e1480a66db9782a3ddd42e66a311e6f))

- *(mcp)* Remove deprecated get_session_by_id and related code ([b4e04f9](https://github.com/lanegrid/agtrace/commit/b4e04f9cbb404e51825fba5055768ca09325dd42))

- *(mcp)* Enhance agent UX with content levels, index stability, semantic previews, and structured actions ([d53b0d1](https://github.com/lanegrid/agtrace/commit/d53b0d1cc32c3e28a93ef6da6bf6003dd346f65d))

- *(test)* Extract and prettify MCP response text content in snapshots ([4efbc1e](https://github.com/lanegrid/agtrace/commit/4efbc1e7feeff84cd8335d26fd9e049954c3989f))

- *(mcp/dto)* Return original models directly to preserve complete session data ([b472ad2](https://github.com/lanegrid/agtrace/commit/b472ad2ba3b7967845d6f554586684b08cc7ab63))

- *(mcp)* Unify dto/view_models into models with presenter layer ([1c467cc](https://github.com/lanegrid/agtrace/commit/1c467ccc60df036a519f98144a1ebc4992bd3098))

- *(mcp)* Consolidate request/response types into tool-specific files under models/types/ ([0122771](https://github.com/lanegrid/agtrace/commit/0122771b620c0532432d1bf5205e56c87e982efc))

- *(mcp)* Merge ToolSummarizer into event_previews for better cohesion ([94e9014](https://github.com/lanegrid/agtrace/commit/94e901498a64f4c1baa90c0460a91d423ab71bf9))

- *(mcp)* Remove _meta and add pagination fields directly to ViewModels ([e78b930](https://github.com/lanegrid/agtrace/commit/e78b9301f27ea428b47357b6af08e941a16c82ca))

- *(mcp)* Remove unused fields and wrapper types ([4b50bfb](https://github.com/lanegrid/agtrace/commit/4b50bfb6e10b92eadb148c6a90e9bd88ef897790))

- *(cli)* Rename serve to mcp serve and consolidate MCP commands ([e465b01](https://github.com/lanegrid/agtrace/commit/e465b0133b8bba3bbf6e1873f02d62d8a16681cb))

- *(cli)* Improve help text with option grouping and examples ([cf37d54](https://github.com/lanegrid/agtrace/commit/cf37d546dde282ac3abfe5fc762ca492f5e581f6))


### Testing

- Add schema verification tests for auto-generated JSON schemas ([2119be3](https://github.com/lanegrid/agtrace/commit/2119be3eaca266c453aaa087749f39455f4c144c))

- *(mcp)* Add snapshot tests for MCP server JSON-RPC protocol ([b0c587e](https://github.com/lanegrid/agtrace/commit/b0c587ebbb2b55088a88db2b3a7e741c4b6ab8b6))


## [0.3.1] - 2026-01-01

### Documentation

- Add Zero-Instrumentation point highlighting auto-discovery of provider logs ([ae2d2d9](https://github.com/lanegrid/agtrace/commit/ae2d2d9c56310e8b1a4061aba0039ebff4eae521))

- Add SDK badge (npm for CLI, crates.io for SDK) ([fcb34e0](https://github.com/lanegrid/agtrace/commit/fcb34e0f251e5ae7d689168c269df763ba12ad22))

- Update SDK version references from 0.2 to 0.3 ([8192c38](https://github.com/lanegrid/agtrace/commit/8192c38b21acb7eebf755f3759424a4cc8363aaf))


### Miscellaneous Tasks

- Add GitHub issue templates for bug reports, feature requests, and provider support ([80f226d](https://github.com/lanegrid/agtrace/commit/80f226d8f9441762a065dc919ee5276ae4fa8cb1))

- Add GitHub label configuration and setup script ([bdf1407](https://github.com/lanegrid/agtrace/commit/bdf140748d3b0efb60880b194fd971ef75d9ad82))


## [0.3.1] - 2026-01-01

### Documentation

- Add Zero-Instrumentation point highlighting auto-discovery of provider logs ([ae2d2d9](https://github.com/lanegrid/agtrace/commit/ae2d2d9c56310e8b1a4061aba0039ebff4eae521))

- Add SDK badge (npm for CLI, crates.io for SDK) ([fcb34e0](https://github.com/lanegrid/agtrace/commit/fcb34e0f251e5ae7d689168c269df763ba12ad22))

- Update SDK version references from 0.2 to 0.3 ([8192c38](https://github.com/lanegrid/agtrace/commit/8192c38b21acb7eebf755f3759424a4cc8363aaf))


### Miscellaneous Tasks

- Add GitHub issue templates for bug reports, feature requests, and provider support ([80f226d](https://github.com/lanegrid/agtrace/commit/80f226d8f9441762a065dc919ee5276ae4fa8cb1))

- Add GitHub label configuration and setup script ([bdf1407](https://github.com/lanegrid/agtrace/commit/bdf140748d3b0efb60880b194fd971ef75d9ad82))


## [0.3.1] - 2026-01-01

### Documentation

- Add Zero-Instrumentation point highlighting auto-discovery of provider logs ([ae2d2d9](https://github.com/lanegrid/agtrace/commit/ae2d2d9c56310e8b1a4061aba0039ebff4eae521))

- Add SDK badge (npm for CLI, crates.io for SDK) ([fcb34e0](https://github.com/lanegrid/agtrace/commit/fcb34e0f251e5ae7d689168c269df763ba12ad22))

- Update SDK version references from 0.2 to 0.3 ([8192c38](https://github.com/lanegrid/agtrace/commit/8192c38b21acb7eebf755f3759424a4cc8363aaf))


### Miscellaneous Tasks

- Add GitHub issue templates for bug reports, feature requests, and provider support ([80f226d](https://github.com/lanegrid/agtrace/commit/80f226d8f9441762a065dc919ee5276ae4fa8cb1))

- Add GitHub label configuration and setup script ([bdf1407](https://github.com/lanegrid/agtrace/commit/bdf140748d3b0efb60880b194fd971ef75d9ad82))


## [0.3.1] - 2026-01-01

### Documentation

- Add Zero-Instrumentation point highlighting auto-discovery of provider logs ([ae2d2d9](https://github.com/lanegrid/agtrace/commit/ae2d2d9c56310e8b1a4061aba0039ebff4eae521))

- Add SDK badge (npm for CLI, crates.io for SDK) ([fcb34e0](https://github.com/lanegrid/agtrace/commit/fcb34e0f251e5ae7d689168c269df763ba12ad22))

- Update SDK version references from 0.2 to 0.3 ([8192c38](https://github.com/lanegrid/agtrace/commit/8192c38b21acb7eebf755f3759424a4cc8363aaf))


### Miscellaneous Tasks

- Add GitHub issue templates for bug reports, feature requests, and provider support ([80f226d](https://github.com/lanegrid/agtrace/commit/80f226d8f9441762a065dc919ee5276ae4fa8cb1))

- Add GitHub label configuration and setup script ([bdf1407](https://github.com/lanegrid/agtrace/commit/bdf140748d3b0efb60880b194fd971ef75d9ad82))


## [0.3.0] - 2026-01-01

### Bug Fixes

- *(types)* Add missing server and tool fields to McpArgs test initialization ([11d6e65](https://github.com/lanegrid/agtrace/commit/11d6e65db8558f0198f5811dff1a6e1f1b798d08))

- *(codex)* Make exit code regex case-insensitive to correctly detect errors ([db204f9](https://github.com/lanegrid/agtrace/commit/db204f9b72d9e8903d18bd1ce9dc24293963f319))

- *(providers)* Resolve all clippy lint warnings and update TokenUsagePayload schema usage ([032a95a](https://github.com/lanegrid/agtrace/commit/032a95a1a4e9db0d3eba498a6ce9bb7431383f6c))

- *(watch)* Remove hardcoded context window fallback in TUI to match console behavior ([5432bc7](https://github.com/lanegrid/agtrace/commit/5432bc783e92fa3488fa191a7e0699f537a117ff))

- *(watch)* Use assembled session for accurate cumulative token display in TUI ([0f9d292](https://github.com/lanegrid/agtrace/commit/0f9d292c9dd0a208b46071c072ec2f98e19b0d04))

- *(watch)* Use same cumulative token logic as SATURATION HISTORY in dashboard ([4862b92](https://github.com/lanegrid/agtrace/commit/4862b92c4ca44947be6f0cf53f84b682dfeae024))

- Correct token double-counting in extract_state_updates (fresh_input should use uncached only) ([f9f4431](https://github.com/lanegrid/agtrace/commit/f9f4431faaf55a0baa9f8627d54d1cf22a73e640))


### Documentation

- Add comprehensive doc comments to SDK re-exported types ([9db448b](https://github.com/lanegrid/agtrace/commit/9db448b6204ad68db08e245e810188cf52b24145))

- *(providers)* Add token conversion rationale for each provider (Claude/Codex/Gemini) ([090826c](https://github.com/lanegrid/agtrace/commit/090826c5779941f1929d913ce9a5095bbc98ac5c))


### Features

- *(sdk)* Add tool_call_stats example for analyzing tool usage across sessions ([598b74a](https://github.com/lanegrid/agtrace/commit/598b74ad710594529ae6c3fd449d42b93f82178e))

- *(sdk)* Refactor tool_call_stats to show detailed per-provider statistics ([abc9c5f](https://github.com/lanegrid/agtrace/commit/abc9c5f5e72d687e5f0999571ff979f7bbde35f4))

- *(sdk)* Add normalized ToolKind statistics to tool_call_stats example ([8f5cffd](https://github.com/lanegrid/agtrace/commit/8f5cffd7385dfad2cd2d7c084a47bbdf81d52f7b))

- *(sdk)* Add Execute command statistics to tool_call_stats example ([1b9fd49](https://github.com/lanegrid/agtrace/commit/1b9fd49afcb3572b25558dbc46b30cf5e6156c64))

- *(sdk)* Add per-tool-name Execute command breakdown in tool_call_stats ([625ab94](https://github.com/lanegrid/agtrace/commit/625ab949dbec48803ae2583c5720f0938074563a))

- *(sdk)* Add command pattern statistics to tool_call_stats (reveals sed usage in codex) ([c98ff8d](https://github.com/lanegrid/agtrace/commit/c98ff8dfd79d0dfe1a919180d96d0670d17d2b12))

- *(codex)* Classify read-oriented shell commands as Read instead of Execute ([408b4b8](https://github.com/lanegrid/agtrace/commit/408b4b869d3b62f2bbd67db064b6e40c366d1c17))

- *(gemini)* Detect MCP tools via display_name pattern instead of mcp__ prefix ([cb1b83f](https://github.com/lanegrid/agtrace/commit/cb1b83ffd423ec92c871395aa76ef3d12d996163))

- *(codex)* Classify pattern search commands (rg/grep) as Search instead of Read ([27de169](https://github.com/lanegrid/agtrace/commit/27de169379d4b7164277ff88bd0fabe3775311ca))

- *(sdk)* Add provider efficiency comparison example ([37f3350](https://github.com/lanegrid/agtrace/commit/37f335004b5c78a7795f33e6ea62d8436f9219d4))

- *(engine)* Migrate to ContextWindowUsage with last-wins semantics for turn aggregation ([9e433b2](https://github.com/lanegrid/agtrace/commit/9e433b278710b1fbadab86399b638e4e21668c87))


### Miscellaneous Tasks

- Move debug examples to sdk/examples/debug and gitignore them ([d3ca56e](https://github.com/lanegrid/agtrace/commit/d3ca56ecd9634e45af8383833d8de9a2089a76a8))

- Remove debug script ([99be7ca](https://github.com/lanegrid/agtrace/commit/99be7caf71fc722c817982f254a0956aaf560df8))


### Refactor

- Move AgentSession and related types from agtrace-engine to agtrace-types ([cd7ed18](https://github.com/lanegrid/agtrace/commit/cd7ed18f402dc9b26f07c3eabd89df6ad137376e))

- Extract compute_turn_metrics to SessionAnalysisExt in agtrace-engine ([99ecfeb](https://github.com/lanegrid/agtrace/commit/99ecfebeb6060f4b755e349655e7d7c9896bb350))

- Move MCP tool name parsing from agtrace-types to agtrace-providers ([c86807f](https://github.com/lanegrid/agtrace/commit/c86807f286b6e0039fd4e64820b4c590ab90b772))

- Add MCP tool name parsing to Codex provider ([30eb2d9](https://github.com/lanegrid/agtrace/commit/30eb2d98c9fafb9cabaf65efecef052a144ec416))

- Add server and tool fields to McpArgs for structured MCP data ([6598568](https://github.com/lanegrid/agtrace/commit/65985684c92a13b742e816df354f7598864cbf62))

- Move MCP normalization to provider mappers and deprecate normalize_tool_call (resolves #26) ([0345d6e](https://github.com/lanegrid/agtrace/commit/0345d6e3d9804c43e8d20cb642ff6d9ecd92e473))

- Change SessionFilter.limit to Option<usize> for unlimited session queries and add provider statistics to tool_call_stats example ([7b79cb7](https://github.com/lanegrid/agtrace/commit/7b79cb7a915161762395f4e00ce4e7d7a7f9124a))

- *(providers)* Use trait-based ProviderAdapter in fixture generation example ([0972303](https://github.com/lanegrid/agtrace/commit/0972303479eb55b040756e6f9d651a6174d635e5))

- *(types)* Normalize TokenUsage schema across providers (Claude/Codex/Gemini) ([5fd2334](https://github.com/lanegrid/agtrace/commit/5fd2334001367096fa41cfe5acf017e0360325c6))

- *(engine)* Remove providers dependency from tests to fix layer violation ([9c7a08a](https://github.com/lanegrid/agtrace/commit/9c7a08a1ae02bc0934bc361be6a58b826d700b55))

- Restore original token aggregation logic while preserving TokenUsagePayload schema ([ce0e162](https://github.com/lanegrid/agtrace/commit/ce0e162450b3af7ed16aa23fe84e00e6ec923122))


### Testing

- Regenerate fixtures and update snapshots for TokenUsagePayload schema changes ([1f71815](https://github.com/lanegrid/agtrace/commit/1f7181537faaafc80246e04c843c000bf9145b00))


## [0.2.1] - 2025-12-31

### Bug Fixes

- Add missing description to agtrace-core for crates.io publish ([06907c5](https://github.com/lanegrid/agtrace/commit/06907c58cda845cacd5f30a9c9663503f65e1ff1))

- Make README version check dynamic for major.minor versions ([428d0a3](https://github.com/lanegrid/agtrace/commit/428d0a34399a3c5a6d5f6c7025e5ba2eb859823a))


### Documentation

- Update README version references from 0.1 to 0.2 ([7f073b7](https://github.com/lanegrid/agtrace/commit/7f073b7ef2c8bd3eded3eca94bc8430c3b8ae733))


## [0.2.0] - 2025-12-31

### Documentation

- Update all references from ~/.agtrace to XDG data directory paths ([e6bbf10](https://github.com/lanegrid/agtrace/commit/e6bbf102d1fed32a4fab8c35b32366d868dc2c32))

- Add quickstart example and improve README SDK usage guide ([cca3c4e](https://github.com/lanegrid/agtrace/commit/cca3c4e821d7da058836423c21eccc596099272d))

- Establish rustdoc as source of truth for SDK usage examples ([c16a0d3](https://github.com/lanegrid/agtrace/commit/c16a0d3f554f4da3d43fa4074359b762c6b35f8f))

- Update AGENTS.md with engine domain and crate dependencies ([5cfaa84](https://github.com/lanegrid/agtrace/commit/5cfaa84f0571e64b0569425836912a21a99b6108))

- Update AGENTS.md with crate design principles and dependency rules ([aaa73f7](https://github.com/lanegrid/agtrace/commit/aaa73f701c8d377c11a33a76005f17cf1901b633))


### Features

- Add auto-initialization to SDK Client::connect() for seamless workspace setup (fixes #17) ([7eb8971](https://github.com/lanegrid/agtrace/commit/7eb8971c855ebb897bbb831c46ff3b453ad6e21e))

- *(sdk)* Implement explicit project scope API for SessionFilter (resolves #24) ([f6a9894](https://github.com/lanegrid/agtrace/commit/f6a9894f63541dbe5e0addcd324659ea5f45fdcf))

- Automate SDK README generation with cargo-rdme ([6defa48](https://github.com/lanegrid/agtrace/commit/6defa48a829c8461eeaee2c66a82d688dce5bd8b))


### Miscellaneous Tasks

- Apply clippy fixes to runtime layer ([a3ce598](https://github.com/lanegrid/agtrace/commit/a3ce59886e3c3eaf5bba113e00b74be51bce4b2c))

- Apply cargo fmt to workspace ([6b14f17](https://github.com/lanegrid/agtrace/commit/6b14f17b56633b3c3a70877ba531e3c2ff399c7c))

- Increment SCHEMA_VERSION to 2 to trigger re-index ([c3a532d](https://github.com/lanegrid/agtrace/commit/c3a532dee09d9f2d9d918cef137a5869993c1293))


### Refactor

- Replace anyhow with structured errors in index and providers layers ([aaa1a39](https://github.com/lanegrid/agtrace/commit/aaa1a3908ce608f57d3c139e842405bdd3d4f3af))

- Replace anyhow with structured errors in runtime layer ([a4cb1d8](https://github.com/lanegrid/agtrace/commit/a4cb1d8d47e3a23d3b7c40ba42063143d40d5518))

- Replace anyhow with structured errors in SDK layer ([b529fe3](https://github.com/lanegrid/agtrace/commit/b529fe388c908bc317c6ef8cbe2e439585fbd8f7))

- Replace anyhow with structured errors in types layer ([27596fd](https://github.com/lanegrid/agtrace/commit/27596fda6d5d5432ea075871fc3e31e192831c49))

- Convert SDK and runtime to async-first architecture ([44ff87a](https://github.com/lanegrid/agtrace/commit/44ff87a933a4e91167780b4eeb6b5b34232f4a9e))

- Add ClientBuilder with XDG path resolution and convert LiveStream to Stream trait ([b4d072d](https://github.com/lanegrid/agtrace/commit/b4d072d761662cdac8790ebdd252841900a5f246))

- Consolidate XDG path resolution logic in runtime, expose via SDK utils ([012349b](https://github.com/lanegrid/agtrace/commit/012349be99d1edc715eb6a2379971da397b2b2f9))

- Consolidate path utilities into new agtrace-core infrastructure crate (fixes #19) ([72a5ba0](https://github.com/lanegrid/agtrace/commit/72a5ba093cf19fd043ca4508502261ace9f363bf))

- Remove deprecated methods from agtrace-sdk ([19eadcf](https://github.com/lanegrid/agtrace/commit/19eadcf8bb15f5ba93cabb9ba53f92714e844b48))

- Migrate integration tests from CLI to SDK for faster type-safe testing ([ef6a7bc](https://github.com/lanegrid/agtrace/commit/ef6a7bc95b5f6983345c442159cdbffe9bfecb20))

- *(sdk)* Re-export resolve_workspace_path directly from core for consistency ([5a47d7b](https://github.com/lanegrid/agtrace/commit/5a47d7b536b16952959e09bc403646b76ddc95cd))

- Move domain from runtime to engine with dependency inversion ([bae964d](https://github.com/lanegrid/agtrace/commit/bae964db4aca70d832161d25242d3dc030a4dd4c))


## [0.1.15] - 2025-12-31

### Bug Fixes

- Remove duplicate v0.1.12 section in CHANGELOG and update all past releases ([a8ea559](https://github.com/lanegrid/agtrace/commit/a8ea55950c0d037649546ff8c9e26420e9c738c0))


### Documentation

- Add README doctest validation to prevent stale examples and version drift ([faf5b88](https://github.com/lanegrid/agtrace/commit/faf5b88c23d73527dd250a36529dafdd1b80d116))

- Add comprehensive releaser skill with safety procedures and rollback script ([517cfc3](https://github.com/lanegrid/agtrace/commit/517cfc324337dcc44bd9959b80ec3ed5f5937ad0))


### Features

- Add prepare-release script for automated release workflow ([fdf66ed](https://github.com/lanegrid/agtrace/commit/fdf66eddd182edcf191bed0e7ca4ea175824b291))

- Use CHANGELOG.md content for GitHub release notes instead of cargo-dist default ([4201843](https://github.com/lanegrid/agtrace/commit/42018437f2a3da347ee1444501273699a0dc6dfd))


### Miscellaneous Tasks

- Remove releaser skill ([495c1b8](https://github.com/lanegrid/agtrace/commit/495c1b820f24119e2565cc9c38d246a697ca93cb))


### Refactor

- Convert releaser skill to standard directory structure with frontmatter ([13a9ff1](https://github.com/lanegrid/agtrace/commit/13a9ff19545c917ecd4f9eb585b0e401a8e1fc80))

- Speed up demo video intro by removing comments and faster typing ([7997475](https://github.com/lanegrid/agtrace/commit/79974752f9b1ac1f22dcd7b85b75b897a24d47ca))


## [0.1.14] - 2025-12-31

### Bug Fixes

- *(sdk)* Make SessionFilter::default() equivalent to new() with reasonable limit ([2f1f481](https://github.com/lanegrid/agtrace/commit/2f1f481e28c76118a4821f0290f47221bcf96540))


### Documentation

- Complete SDK-CLI refactor ExecPlan with outcomes and retrospective ([2f4e456](https://github.com/lanegrid/agtrace/commit/2f4e45604fbefe55dd001021ada08b86b2119170))

- Remove completed SDK-CLI refactor planning documents ([cb2a668](https://github.com/lanegrid/agtrace/commit/cb2a6689f4a309e088f779e38c1116ecde6b0305))


### Refactor

- *(cli)* Migrate all handlers to use agtrace-sdk instead of internal crates ([548a7b7](https://github.com/lanegrid/agtrace/commit/548a7b7c53f46d63b9a552fa6de8946bf3a8bc4c))

- *(cli)* Replace internal crate imports with SDK in presentation layer ([2e0da9a](https://github.com/lanegrid/agtrace/commit/2e0da9acdd9b8ff3b4f3cfd7a05dcf141c57e513))

- *(cli)* Achieve strict SDK-only dependency by eliminating all internal crate usage ([22a0590](https://github.com/lanegrid/agtrace/commit/22a059038689dd80b7c1d51ead50fa1cf5e826e7))

- *(sdk)* Introduce utils module for low-level API, remove internal re-exports, add SessionHandle::from_events() ([4e48516](https://github.com/lanegrid/agtrace/commit/4e485169ad818ffa8a91ebce3c1a10c11cef717f))


### Testing

- Migrate integration tests to use agtrace-sdk types ([aeb458e](https://github.com/lanegrid/agtrace/commit/aeb458ee8d259200335f52daed5b97fc722fa10c))


## [0.1.13] - 2025-12-30

### Features

- *(sdk)* Add agtrace-sdk facade for building observability tools ([1f99847](https://github.com/lanegrid/agtrace/commit/1f99847bc2e081cabfe87eaf6d6135f946e33489))

- *(sdk)* Add working examples demonstrating SDK usage (connection, analysis, watch) ([d1cc697](https://github.com/lanegrid/agtrace/commit/d1cc69760fc14eafcb482f339a1d5562c8304857))

- *(sdk)* Implement Iterator trait for LiveStream and structured Insight type with Severity ([c8e38f0](https://github.com/lanegrid/agtrace/commit/c8e38f0ef846433669558e310da2acc5f61f331e))


### Bug Fixes

- *(sdk)* Remove unused mut and Duration import from watch_events example ([1adfd62](https://github.com/lanegrid/agtrace/commit/1adfd6296349c5eb9a9b4b8bdb5d02a77ea8a6f5))


### Documentation

- Reframe agtrace as observability platform with SDK and CLI applications ([891caad](https://github.com/lanegrid/agtrace/commit/891caad8c6290f0a7767e71965ef93b1c06baea9))

- Improve README with Iterator usage, workspace context, and Mermaid architecture diagram ([1910bf0](https://github.com/lanegrid/agtrace/commit/1910bf0ee907951bfcca230c240ab55820d808ec))


### Refactor

- Replace String-based project_hash with type-safe ProjectHash throughout codebase ([958233b](https://github.com/lanegrid/agtrace/commit/958233bb737068b1f270cee7d199fca8331f83e6), [2261ac1](https://github.com/lanegrid/agtrace/commit/2261ac1d7e89199cfaf001d0b5d71a3abac36848), [04cc841](https://github.com/lanegrid/agtrace/commit/04cc84124249ddb9f4eb16e6e7abc0a2e0f050cb))

- Migrate from context_window_tokens() to total_tokens() and remove legacy methods ([d59ebec](https://github.com/lanegrid/agtrace/commit/d59ebec8681af13eb9905357acdc496a5bd4fda4), [f2870e6](https://github.com/lanegrid/agtrace/commit/f2870e68e3ceb22d3fa009f3e3bdda711daf22eb), [0d62675](https://github.com/lanegrid/agtrace/commit/0d6267522a326e9aea30983a27be82e0d6a88c8f))

- *(agtrace-types)* Reorganize into domain/, event/, and tool/ modules ([af061aa](https://github.com/lanegrid/agtrace/commit/af061aaab26847291386a4fe5c1d0195439cb8d6))

- *(agtrace-index)* Reorganize db.rs into modular structure (records, schema, queries) ([dec1721](https://github.com/lanegrid/agtrace/commit/dec17219c10daf0bbc61d6a25484268388fff7be))


## [0.1.12] - 2025-12-29

### Bug Fixes

- *(cli)* Remove duplicate verbose argument in doctor run command ([17d08d7](https://github.com/lanegrid/agtrace/commit/17d08d715eb1093e30632069731c189deda9bb7a))

- *(watch)* Filter events by project hash to respect project isolation (#12) ([9f5c42a](https://github.com/lanegrid/agtrace/commit/9f5c42a942808eed7a688e00cee67e17d4d34253))


### Documentation

- *(cli)* Improve help text with user-friendly descriptions and quick start guide ([8696800](https://github.com/lanegrid/agtrace/commit/869680060e934196dadd194f67478c83ddbbdcfd))


### Features

- *(tui)* Add contextual waiting state hints with actionable commands and exact directory match requirement ([a250d81](https://github.com/lanegrid/agtrace/commit/a250d815fb124d8e42ea0b8cd51297b4df32e21c))


### Miscellaneous Tasks

- Update CHANGELOG for v0.1.11 ([ae51b18](https://github.com/lanegrid/agtrace/commit/ae51b18cd43a604aa02921670df28d5a008da595))

- Bump version to 0.1.11 ([ce33554](https://github.com/lanegrid/agtrace/commit/ce3355441efafc1536f0750a2bcee11ce2582e82))


## [0.1.11] - 2025-12-29

### Bug Fixes

- *(cli)* Remove duplicate verbose argument in doctor run command ([17d08d7](https://github.com/lanegrid/agtrace/commit/17d08d715eb1093e30632069731c189deda9bb7a))

- *(watch)* Filter events by project hash to respect project isolation (#12) ([9f5c42a](https://github.com/lanegrid/agtrace/commit/9f5c42a942808eed7a688e00cee67e17d4d34253))


### Documentation

- *(cli)* Improve help text with user-friendly descriptions and quick start guide ([8696800](https://github.com/lanegrid/agtrace/commit/869680060e934196dadd194f67478c83ddbbdcfd))


### Features

- *(tui)* Add contextual waiting state hints with actionable commands and exact directory match requirement ([a250d81](https://github.com/lanegrid/agtrace/commit/a250d815fb124d8e42ea0b8cd51297b4df32e21c))


## [0.1.10] - 2025-12-29

### Documentation

- Split README into focused documentation structure (motivation, getting-started, commands, architecture, faq, providers)
- Consolidate provider documentation with accurate log paths
- Simplify documentation by removing redundant sections
- Add cargo install option to README

## [0.1.9] - 2025-12-29

### Bug Fixes

- Pass project_root to console mode handlers for correct display ([dc2c5c9](https://github.com/lanegrid/agtrace/commit/dc2c5c9751c7692049fa3b2dc99a5ecadbfb36b9))

- Watch should scan selected provider only, not all providers ([57a464f](https://github.com/lanegrid/agtrace/commit/57a464ff07246a08017ae16a0333bb5f93592a0e))

- Scan all providers before selecting most recent session for watch ([ec8c4b0](https://github.com/lanegrid/agtrace/commit/ec8c4b002efb5a4cc7a236882d85cff9dde92041))


### Features

- Display project_root and log_path in watch stream header ([678a606](https://github.com/lanegrid/agtrace/commit/678a6060661fab5a2a4ca28aaa5eaec093573da5))

- Enable cross-provider session switching in watch mode by tracking latest_mod_time ([3c40948](https://github.com/lanegrid/agtrace/commit/3c40948a2c98fb0dfe2ad4a4d4e46a37496f96c3))


### Miscellaneous Tasks

- Apply cargo fmt to demo.rs ([babfbde](https://github.com/lanegrid/agtrace/commit/babfbde99ba0c6bfb367193e0b3b79e610462ec0))


### Refactor

- Separate project_root and log_path in SessionState for accurate display ([298a649](https://github.com/lanegrid/agtrace/commit/298a649bd07dd13c2ecc6289aa033052ecd5156b))

- Unify console and TUI view models for watch mode ([bd6728d](https://github.com/lanegrid/agtrace/commit/bd6728da32c7e33a69048ab525e6e9cc12b128ef))

- Consolidate mod_time logic and add layer violation TODOs ([f104fe1](https://github.com/lanegrid/agtrace/commit/f104fe1fc4893c7d212920382bf00eebfc686090))


### Testing

- Add cross-provider session switching integration test ([200a00f](https://github.com/lanegrid/agtrace/commit/200a00f4631a695a92b1f6e4ca827ae4fec43d8c))


## [0.1.8] - 2025-12-28

### Documentation

- Rewrite README to emphasize observability layer and compaction behavior ([c8a669f](https://github.com/lanegrid/agtrace/commit/c8a669fac6db12214febc0796f3b32b62ce5d032))

- Rewrite README to emphasize observability layer and compaction behavior ([8fafcf4](https://github.com/lanegrid/agtrace/commit/8fafcf4680ba80e93decb634ed3d348cee8034a1))

- Clarify CWD-scoped monitoring and improve Quick Start workflow ([63e5c38](https://github.com/lanegrid/agtrace/commit/63e5c38e8d2c7d4e4d56576054055615c89237c7))

- Use GitHub raw content URLs for images and move demo.gif to docs/assets ([d4dbda6](https://github.com/lanegrid/agtrace/commit/d4dbda623e32e18cb8519bcced6dd22a70ec2e2d))


### Miscellaneous Tasks

- Remove PROGRESS.md ([0e728f5](https://github.com/lanegrid/agtrace/commit/0e728f5c627988d64e93cd640c6a30f73153c3bd))


## [0.1.7] - 2025-12-28

### Features

- Add demo mode to showcase TUI without requiring local logs ([a7f3261](https://github.com/lanegrid/agtrace/commit/a7f3261))

### Bug Fixes

- Change turn percentage display from cumulative to delta (incremental) ([65eeaa5](https://github.com/lanegrid/agtrace/commit/65eeaa5))
- Preserve all events in demo to prevent turn count reduction ([1bbe397](https://github.com/lanegrid/agtrace/commit/1bbe397))
- Link demo notifications to progress bar percentage instead of event index ([07f0577](https://github.com/lanegrid/agtrace/commit/07f0577))
- Unify progress bar calculation to include both input and output tokens ([49e0b5b](https://github.com/lanegrid/agtrace/commit/49e0b5b))
- Add context window limit enforcement to demo token generation ([f6771c5](https://github.com/lanegrid/agtrace/commit/f6771c5))
- Update demo model name and prevent context window overflow ([c708a9c](https://github.com/lanegrid/agtrace/commit/c708a9c))
- Assemble session from events to display turn data in demo mode ([7f77b87](https://github.com/lanegrid/agtrace/commit/7f77b87))
- Correct provider default log paths in help text ([0d6f5b8](https://github.com/lanegrid/agtrace/commit/0d6f5b8))

### Refactoring

- Unify --source option to --provider across CLI ([657cd40](https://github.com/lanegrid/agtrace/commit/657cd40))
- Rename source to provider in internal API ([b7ab5a4](https://github.com/lanegrid/agtrace/commit/b7ab5a4))
- Centralize CLI command hints to prevent duplication and typos ([6188a34](https://github.com/lanegrid/agtrace/commit/6188a34))
- Add scenario builder pattern and expand demo to 7 turns with 100+ events ([7917ffb](https://github.com/lanegrid/agtrace/commit/7917ffb))
- Unify token usage logic by using engine's extract_state_updates in demo ([a79f5b9](https://github.com/lanegrid/agtrace/commit/a79f5b9))
- Remove hardcoded context limit in demo, use configurable constant ([9277085](https://github.com/lanegrid/agtrace/commit/9277085))

### Documentation

- Add VHS demo gif and agtrace demo command documentation ([ea15513](https://github.com/lanegrid/agtrace/commit/ea15513))
- Regenerate demo.gif with cargo-installed agtrace v0.1.6 ([ef0b1c7](https://github.com/lanegrid/agtrace/commit/ef0b1c7))
- Reduce demo.gif size for better readability (1200x700) ([d328478](https://github.com/lanegrid/agtrace/commit/d328478))
- Organize demo generation scripts into scripts/demo directory ([acd1d46](https://github.com/lanegrid/agtrace/commit/acd1d46))
- Increase demo.gif font size for better readability (FontSize 18) ([508d1c1](https://github.com/lanegrid/agtrace/commit/508d1c1))
- Improve CLI help text and command descriptions for better UX ([0ae8b91](https://github.com/lanegrid/agtrace/commit/0ae8b91))
- Remove unnecessary documents ([98bb313](https://github.com/lanegrid/agtrace/commit/98bb313))
- Add centered logo to README header ([0327721](https://github.com/lanegrid/agtrace/commit/0327721))
- Add crates.io badge and cargo install instructions ([2908ebb](https://github.com/lanegrid/agtrace/commit/2908ebb))

## [0.1.6] - 2025-12-27

### Infrastructure

- Rename CLI package from `agtrace-cli` to `agtrace` for better discoverability on crates.io
- Add crates.io publishing automation to GitHub Actions release workflow
- Mark internal crates with `agtrace-internal` keyword to prevent accidental usage
- Add package metadata (categories, keywords, readme) for crates.io optimization

## [0.1.5] - 2025-12-27

### Bug Fixes

- Correct init command hints to suggest 'watch' and 'session list' instead of non-existent 'list' ([bf1ce4f](https://github.com/lanegrid/agtrace/commit/bf1ce4f))


### Documentation

- Reorder Quick Start to emphasize 'watch' workflow as primary use case ([bd9e03c](https://github.com/lanegrid/agtrace/commit/bd9e03c))

- Clarify Quick Start workflow with explicit agent launch steps and no-integration requirement ([09d7e29](https://github.com/lanegrid/agtrace/commit/09d7e29))

- Update screenshot to use Claude-specific dashboard image ([e504ee4](https://github.com/lanegrid/agtrace/commit/e504ee4))


## [0.1.4] - 2025-12-27

### Bug Fixes

- Watch command now selects provider with most recent session (issue #6) ([6802b22](https://github.com/lanegrid/agtrace/commit/6802b22))

- Perform session indexing during init before counting sessions (issue #5) ([7cf2bae](https://github.com/lanegrid/agtrace/commit/7cf2bae))

- Implement provider filtering in index commands ([72cd3af](https://github.com/lanegrid/agtrace/commit/72cd3af))

- Canonicalize paths in project_hash_from_root and add comprehensive integration tests ([834564c](https://github.com/lanegrid/agtrace/commit/834564c))


### Testing

- Add failing test that documents issue #5 bug (init reports 0 sessions before indexing) ([2b1a3d2](https://github.com/lanegrid/agtrace/commit/2b1a3d2))

- Add provider filtering tests with provider-agnostic test infrastructure ([ae99ec1](https://github.com/lanegrid/agtrace/commit/ae99ec1))


### Documentation

- Add test-driven bug fix strategy to AGENTS.md ([9d03968](https://github.com/lanegrid/agtrace/commit/9d03968))

- Update progress and bug status - all 21 integration tests passing ([5c51beb](https://github.com/lanegrid/agtrace/commit/5c51beb))


## [0.1.3] - 2025-12-25

### Bug Fixes

- Compute project_hash from SessionIndex.project_root instead of hardcoded 'unknown' (fixes #1) (#2) ([aca561a](https://github.com/lanegrid/agtrace/commit/aca561a282968cce9163e48fc7bedc0fe0fb938c))

- Ensure_index_is_fresh derives project_hash from cwd and respects --all-projects flag ([5615036](https://github.com/lanegrid/agtrace/commit/5615036a49692f72941d5a352f5663cb1c759339))


### Testing

- Add comprehensive integration tests for edge cases and project isolation ([bf75867](https://github.com/lanegrid/agtrace/commit/bf75867643157744a060a6f8f00f3af16b9a30f8))

- Fix project isolation tests with proper cwd/sessionId replacement to catch real bugs ([dacfc2f](https://github.com/lanegrid/agtrace/commit/dacfc2faca4fc0b33c833ddd3d3deeba90265397))

- Fix compilation errors in scan_legacy_project_hash_test and improve test helper formatting ([0d474a4](https://github.com/lanegrid/agtrace/commit/0d474a437a9bb26dabda3403887759b5ac035faf))


The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] - 2025-12-25

### Added

- Initial public release on crates.io and npm
- Core library APIs for AI agent log analysis
  - Multi-provider normalization (Claude Code, Codex, Gemini)
  - Session parsing and event stream processing
  - SQLite-based indexing with schema-on-read architecture
- CLI commands:
  - `init` - Initialize workspace and detect providers
  - `list` - Show session history
  - `show` - Display session details with token usage
  - `watch` - Real-time TUI dashboard for live sessions
  - `doctor` - Verify configuration and database integrity
  - `lab grep` - Search across sessions with regex and JSON output
- Features:
  - Live context window monitoring
  - Token usage tracking (input/output/cache/reasoning)
  - Provider-agnostic event normalization
  - Local-only processing (no cloud dependencies)
  - Zero-overhead monitoring with Rust performance

### Fixed

- Prevent panic when session_id is shorter than 8 characters in watch mode

## [0.1.1] - 2025-12-25

_Internal development release - not published to crates.io or npm_

## [0.1.0] - 2025-12-25

_Internal development release - not published to crates.io or npm_
