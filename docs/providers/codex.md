# Codex Provider

Provider-specific details for Codex (OpenAI).

## Log Location

Codex stores session logs at:
```
~/.codex/sessions/<session-id>/stream.jsonl
```

## Event Format

Codex uses a JSONL (newline-delimited JSON) format where each line represents an event in the session timeline.

## Supported Features

- Context window tracking
- Tool call normalization
- Turn/step reconstruction
- Token counting and cost estimation

## Provider-Specific Notes

(This section will be expanded as provider-specific behaviors and quirks are documented)
