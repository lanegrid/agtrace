---
name: agtrace-provider-normalization
description: Investigate AI agent provider tool schemas (Claude Code, Codex, Gemini) and design unified domain abstractions in agtrace architecture.
---

# agtrace Provider Normalization

1. agtrace-types defines domain models (ToolCallPayload variants + Args structs) with NO knowledge of provider-specific tool names or formats
2. agtrace-providers/<provider> mod consolidates diverse provider-specific tool names/schemas into a small set of unified domain variants (e.g., Claude "Edit" + Gemini "replace" → FileEdit variant)
3. Unmappable tools fallback to Generic variant, with provider-specific semantics hidden in the corresponding <provider> mod
4. Investigate new tools using `lab grep --source PROVIDER --raw` to extract schemas from real data and assess compatibility with existing variants
5. If unifiable, add variant to types and reference from multiple provider mods; if not, keep as Generic or consider provider-specific variant, isolating conversion logic in provider layer
