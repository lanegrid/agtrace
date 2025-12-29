# Frequently Asked Questions

## CWD-Scoped Monitoring

### Why does agtrace use current working directory (cwd) for scoping?

agtrace uses your current working directory as the scope boundary for log discovery and session tracking because:

1. **Simplicity** - Most development workflows are single-project. You're typically working in one project directory at a time.

2. **Isolation** - Sessions from different projects should remain separate. Using cwd as the boundary ensures you only see sessions relevant to your current work.

3. **Consistency with Agent Behavior** - AI coding agents are typically started from a specific project directory. agtrace mirrors this pattern for predictable behavior.

### How do I monitor sessions from a different project?

Simply `cd` to that project's directory and run agtrace commands from there:

```bash
cd /path/to/other/project
agtrace session list
agtrace watch
```

### Can I monitor multiple projects simultaneously?

Yes, but each project needs its own `agtrace watch` instance. Use terminal multiplexers like tmux or split terminals:

```bash
# Terminal 1
cd /path/to/project1
agtrace watch

# Terminal 2
cd /path/to/project2
agtrace watch
```

### What if I use monorepos or nested projects?

agtrace determines the project root by the directory where you run the command. For monorepos:

- Run `agtrace` from the monorepo root to see all sessions in that workspace
- Run `agtrace` from a subdirectory to see only sessions scoped to that subdirectory

Note: agtrace does not support hierarchical project relationships. Each directory is treated as an independent project.

## Installation and Setup

### Do I need to run `agtrace init` for each project?

No. `agtrace init` is a **global, one-time setup** that creates configuration under `~/.agtrace`. You do not need to run it per project.

### Can I use agtrace with npx without installing globally?

Yes. Replace `agtrace` with `npx @lanegrid/agtrace` in all commands:

```bash
npx @lanegrid/agtrace@latest init
npx @lanegrid/agtrace@latest watch
```

However, for best performance and convenience (especially for `watch`), global installation is recommended.

## Data and Privacy

### Does agtrace send data to the cloud?

No. agtrace runs **100% locally**. It reads log files from your local filesystem (e.g., `~/.claude`) and stores metadata in a local SQLite database (`~/.agtrace/agtrace.db`). No data is sent to external servers.

### Where are my logs stored?

agtrace does not copy or move your logs. It reads them from their original provider locations (see [Supported Providers](providers.md)).

The only files agtrace creates are:
- `~/.agtrace/agtrace.db` (metadata index)
- `~/.agtrace/config.toml` (configuration)

### Can I delete the agtrace database?

Yes. The database (`~/.agtrace/agtrace.db`) is disposable and can be rebuilt from the original log files at any time. If you delete it, run `agtrace init` to recreate it.

## Usage

### Why doesn't `agtrace watch` show my session?

Check these common issues:

1. **Wrong directory** - Ensure you're running `agtrace watch` from the same directory where you started your AI coding agent.

2. **Session not started yet** - If the agent hasn't created any log files yet, `watch` will wait in "waiting mode" until it detects a session.

3. **Provider not supported** - Ensure you're using a supported provider (Claude Code, Codex, or Gemini).

### How do I find a session ID?

Use `agtrace session list` to see recent sessions and their IDs:

```bash
agtrace session list
```

### Can I export session data?

Yes. Use the `--json` flag with most commands to export data in JSON format:

```bash
agtrace session show <session-id> --json > session.json
agtrace lab grep "pattern" --json > results.json
```

## Performance

### Why is agtrace slow when I first run it?

agtrace parses logs on demand (schema-on-read). The first time you query a session or run `watch`, it may need to parse large JSONL files. Subsequent queries are faster due to caching.

### Will agtrace slow down my agent?

No. agtrace only reads log files after they're written. It does not intercept or modify agent operations.

## Troubleshooting

### "No sessions found" error

This usually means:
- You haven't run any AI coding agent sessions yet in this project directory
- The logs are in a location agtrace doesn't recognize
- You're running agtrace from a different directory than where the agent was started

Try:
1. `cd` to your project directory
2. Start a new agent session
3. Run `agtrace session list` again

### How do I report a bug or request a feature?

Visit the [GitHub Issues page](https://github.com/lanegrid/agtrace/issues) to report bugs or suggest features.
