---
name: agtrace-analyzer
description: Analyze and visualize agtrace session data. Use when examining agent traces, debugging sessions, comparing session outputs, or investigating agent behavior patterns.
allowed-tools: Bash, Read, Grep
---

# Agtrace Session Analyzer

## Purpose

This skill helps analyze agtrace session data efficiently by automating common analysis patterns and providing structured insights into agent behavior.

## Instructions

### 1. List available sessions

```bash
./target/debug/agtrace list
# or for release build
./target/release/agtrace list
```

### 2. Analyze a specific session

When analyzing a session:

1. Show the session details using `agtrace show <session-id>`
2. Extract key information:
   - Session ID and timestamp
   - Number of interactions
   - Input/output patterns
   - Error messages if any
3. Provide structured summary of findings

### 3. Compare multiple sessions

When comparing sessions:

1. Retrieve details for each session
2. Identify common patterns and differences
3. Highlight anomalies or interesting behaviors

### 4. Search patterns across sessions

Use `grep` to search for specific patterns in session data:

```bash
# Example: Find sessions with specific error patterns
./target/debug/agtrace show <session-id> | grep -i error
```

## Analysis patterns

### Session health check

- Check for error messages
- Verify complete interaction flows
- Identify truncated or incomplete sessions

### Performance analysis

- Count number of interactions per session
- Identify long-running operations
- Compare session durations

### Pattern detection

- Find common input patterns
- Identify recurring errors
- Detect unusual agent behaviors

## Output format

Provide analysis results in this structure:

1. **Session Overview**: ID, timestamp, interaction count
2. **Key Findings**: Important patterns or issues
3. **Recommendations**: Actionable insights if applicable
4. **Raw Data**: Relevant excerpts from session data

## Best practices

- Always verify the build exists before running agtrace commands
- Use `list` to get session IDs before calling `show`
- For multiple sessions, analyze them in parallel when possible
- Focus on actionable insights rather than raw data dumps
