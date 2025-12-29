# Claude Code Provider

Provider-specific details for Claude Code (Anthropic).

## Log Location

Claude Code stores session logs at:
```
~/.claude/sessions/<session-id>/events.jsonl
```

## Event Format

Claude Code uses a JSONL (newline-delimited JSON) format where each line represents an event in the session timeline.

## Supported Features

- Context window tracking
- Tool call normalization
- Turn/step reconstruction
- Token counting and cost estimation

## Provider-Specific Notes

(This section will be expanded as provider-specific behaviors and quirks are documented)
