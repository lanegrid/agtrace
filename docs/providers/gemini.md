# Gemini Provider

Provider-specific details for Gemini (Google).

## Log Location

Gemini stores session logs at:
```
~/.gemini/sessions/<session-id>/events.jsonl
```

## Event Format

Gemini uses a JSONL (newline-delimited JSON) format where each line represents an event in the session timeline.

## Supported Features

- Context window tracking
- Tool call normalization
- Turn/step reconstruction
- Token counting and cost estimation

## Provider-Specific Notes

(This section will be expanded as provider-specific behaviors and quirks are documented)
