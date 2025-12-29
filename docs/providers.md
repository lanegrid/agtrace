# Supported Providers

agtrace supports multiple AI coding agent providers through a unified normalization layer.

## Provider List

| Provider | Description | Default Log Path |
|----------|-------------|------------------|
| **Claude Code** | Claude Code IDE (Anthropic) | `~/.claude/projects` |
| **Codex** | Codex CLI (OpenAI) | `~/.codex/sessions` |
| **Gemini** | Gemini CLI (Google) | `~/.gemini/tmp` |

## How It Works

agtrace automatically detects which provider you're using by:
1. Scanning default log paths for each provider
2. Identifying the provider from the log file structure
3. Normalizing provider-specific events into a unified `AgentEvent` model

## Common Features Across Providers

All providers support:
- Context window tracking
- Tool call normalization
- Turn/step reconstruction
- Token counting and cost estimation

## Provider-Specific Notes

Provider-specific behaviors and quirks will be documented here as they are discovered.
