---
name: agtrace-provider-normalization
description: Investigate AI agent provider tool schemas (Claude Code, Codex, Gemini) and design unified domain abstractions in agtrace architecture.
---

# agtrace Provider Normalization

**IMPORTANT**: Read `docs/provider_tool_normalization.md` for detailed workflow and examples.

## Architecture Principles

1. agtrace-types defines domain models (ToolCallPayload variants + Args structs) with NO knowledge of provider-specific tool names or formats
2. agtrace-providers/<provider> mod consolidates diverse provider-specific tool names/schemas into a small set of unified domain variants (e.g., Claude "Edit" + Gemini "replace" → FileEdit variant)
3. Unmappable tools fallback to Generic variant, with provider-specific semantics hidden in the corresponding <provider> mod
4. Investigate new tools using `lab grep --provider PROVIDER --raw` to extract schemas from real data and assess compatibility with existing variants
5. If unifiable, add variant to types and reference from multiple provider mods; if not, keep as Generic or consider provider-specific variant, isolating conversion logic in provider layer

## Workflow

1. **Schema Investigation**: ALWAYS use `--raw` flag
   ```bash
   ./target/release/agtrace lab grep '"name":"TOOL_NAME"' --raw --limit 5
   ```
   - `--raw` shows both normalized (`content.arguments`) and original provider data (`metadata.payload`)
   - Essential for understanding provider-specific schema and verifying normalization

2. **Define Provider-Specific Struct** in `crates/agtrace-providers/src/<provider>/tools.rs`
   - Exact representation of provider's schema
   - Include parsing logic (`parse()` method)
   - Add tests for parsing

3. **Map to Domain Model** in `crates/agtrace-providers/src/<provider>/normalize.rs`
   - Create `normalize_<provider>_tool_call()` function
   - Map provider tool to appropriate domain variant (FileEdit, FileWrite, etc.)
   - Add tests for normalization

4. **Verify with Real Data**:
   ```bash
   ./target/release/agtrace lab grep '"name":"TOOL_NAME"' --raw --limit 1
   ```
   - Check `content.arguments` (after normalization)
   - Compare with `metadata.payload` (before normalization)
   - Verify extraction/transformation logic

## Reference

See `docs/provider_tool_normalization.md` for:
- Complete case study (Codex `apply_patch`)
- `--raw` vs `--json` comparison
- Step-by-step implementation guide
- Verification checklist
