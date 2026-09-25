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

Yes. `agtrace watch --all-projects` shows the live roots of every project in one tree.
Alternatively, run one `agtrace watch` per project in tmux or split terminals:

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

No. `agtrace init` is a **global, one-time setup** that creates configuration in the system data directory (e.g., `~/Library/Application Support/agtrace` on macOS). You do not need to run it per project.

### Can I use agtrace with npx without installing globally?

Yes. Replace `agtrace` with `npx @lanegrid/agtrace` in all commands:

```bash
npx @lanegrid/agtrace@latest init
npx @lanegrid/agtrace@latest watch
```

However, for best performance and convenience (especially for `watch`), global installation is recommended.

## Data and Privacy

### Does agtrace send data to the cloud?

No. agtrace runs **100% locally**. It reads log files from your local filesystem (e.g., `~/.claude`) and stores metadata in a local SQLite database in the system data directory. No data is sent to external servers.

### Where are my logs stored?

agtrace does not copy or move your logs. It reads them from their original provider locations (see [Supported Providers](providers.md)).

The only files agtrace creates are (in system data directory, e.g., `~/Library/Application Support/agtrace`):
- `agtrace.db` (metadata index)
- `config.toml` (configuration)

### Can I delete the agtrace database?

Yes. The database (`agtrace.db`) is disposable and can be rebuilt from the original log files at any time. If you delete it, run `agtrace init` to recreate it.

## Usage

### Why doesn't `agtrace watch` show my session?

Check these common issues:

1. **Wrong directory** - Ensure you're running `agtrace watch` from the same directory where you started your AI coding agent.

2. **Session not started yet** - If the agent hasn't written a log file yet, `watch` shows an empty tree and adds the session as soon as the file appears.

3. **Session too old** - By default, `watch` shows root sessions that wrote to their log within the last 2 hours or still have a running Claude process. Use `--since 1d` to widen the window, or `--session <id>` to watch one specific session tree.

4. **Provider or version not supported** - agtrace supports Claude Code ≥ 2.1.24x and Codex ≥ 0.153. Older log formats are not decoded.

### Where are subagents and teammates?

`agtrace watch` shows them as children of the session that spawned them. `agtrace session show <id>` prints the session's agent tree, and `agtrace session list --all` includes child sessions. See [Multi-Agent Sessions](multi-agent.md).

### What does "N diag" in the watch status bar mean?

It counts log lines agtrace could not decode: invalid JSON, or records whose shape did not match. Such lines are skipped; the rest of the file is still shown. `agtrace doctor run` lists the affected files.

### How do I find a session ID?

Use `agtrace session list` to see recent sessions and their IDs:

```bash
agtrace session list
```

### Can I export session data?

Yes. Use the `--format json` flag to export data in JSON format:

```bash
agtrace session show <session-id> --format json > session.json
agtrace session list --format json > sessions.json
agtrace lab grep "pattern" --json > results.json
```

## Performance

### Why is agtrace slow when I first run it?

agtrace parses logs on demand (schema-on-read). The first `agtrace init` indexes every log file, but reads only file headers, so it is fast even for large histories. Later index updates skip unchanged files. `watch` reads each tracked file once when it attaches, and after that only reads new bytes.

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
