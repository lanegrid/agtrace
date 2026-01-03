# Agent Anti-Patterns (Learning Phase)

This document tracks interrupt events from AI agent sessions to identify patterns and derive rules iteratively.

## Purpose

Rather than prematurely generalizing into strict rules, we collect evidence from actual user interrupts, identify patterns, and only establish rules when sufficient consistent evidence exists.

## Methodology

1. **Evidence Collection**: Record specific interrupt events with full context
2. **Pattern Candidates**: Group similar evidence (3+ instances → consider rule)
3. **Established Rules**: Move well-supported patterns to CLAUDE.md

## 📊 Evidence Collection

**[E1] Unnecessary manual verification after tests pass**
- Session: `3883a03e`, Turn 1, 2026-01-03T02:47:23Z
- Context: All automated tests (unit + integration) passed successfully
- Agent action: Started writing complex bash script for manual verification (`cat > /tmp/test_sidechain.sh <<'EOF'`)
- Interrupt: User stopped the tool use
- User response: "integration tests を追加すればよい。(understand by tree2md"
- Pattern: Automated tests sufficient → no manual verification needed

**[E2] Complex bash loops failing repeatedly**
- Session: `cc7fe4ef`, Turn 5-6, 2026-01-03T03:34:43Z
- Context: Trying to count tests per file using bash loops
- Agent action: Attempted `for f in tests/*_test.rs; do ... done` → failed 3 times with variable expansion issues
- Interrupt: None, but user intervened
- User response: "first, pwd"
- Pattern: (1) Complex bash constructs unreliable, (2) Verify working directory before operations

**[E3] Using bash commands instead of dedicated tools**
- Session: `3883a03e`, Turn 3, 2026-01-03T02:51:16Z
- Context: Trying to read file contents
- Agent action: Used `cat` in bash
- Interrupt: User stopped the tool use
- User response: "use tool instead of cat"
- Pattern: Prefer Read tool over bash cat

**[E4] Scope creep - changing defaults outside requested scope**
- Session: `dce211e4`, Turn 1, 2026-01-03T10:23:31Z
- Context: User requested "add project root column and header to session list view"
- Agent action: Changed `ViewMode::default()` from `Compact` to `Standard` (not requested), then ran `cargo test`
- Interrupt: User stopped `cargo test`
- User response: "勝手に default 変えてんじゃねーぞ。変えるな変えるな。実装のスコープの影響範囲に抑えろ"
- Follow-up: User had to correct agent's understanding of what needed to be done
- Pattern: Keep implementation changes within requested scope; don't modify defaults or unrelated behavior

## 🔍 Pattern Candidates (3+ evidence → consider rule)

**[P1] Prefer dedicated tools over bash commands** (3 evidence: E2, E3, + existing CLAUDE.md rule)
- Current status: Already documented in CLAUDE.md as existing convention
- Evidence: E2 (bash loops unreliable), E3 (use Read not cat)
- Note: Reinforces existing rule rather than new pattern

**[P2] Stateful shell context awareness** (1 evidence: E2)
- Current status: Needs more evidence
- Hypothesis: Before complex operations in persistent shell, verify context (pwd, tree2md)
- Evidence: E2 only
- Counter-evidence needed: Cases where context verification was unnecessary overhead

**[P3] Test sufficiency judgment** (1 evidence: E1)
- Current status: Needs more evidence - highly context-dependent
- Hypothesis: When all automated tests pass, consider task complete unless explicit verification request
- Evidence: E1 only
- Counter-evidence needed: Cases where manual verification was actually necessary after tests passed
- Challenge: Determining when manual verification adds value vs. when it's redundant

**[P4] Scope creep - avoid unrelated changes** (1 evidence: E4)
- Current status: Needs more evidence
- Hypothesis: Limit implementation changes to what user explicitly requested; avoid modifying defaults, global config, or unrelated behavior
- Evidence: E4 only
- Counter-evidence needed: Cases where proactive changes to defaults/config improved UX without user complaint
- Challenge: Distinguishing necessary refactoring from scope creep

## ✅ Established Rules (migrated to CLAUDE.md)

None yet. Patterns need 3+ consistent evidence before becoming rules.

## Adding New Evidence

When adding new interrupt evidence:

1. Use next sequential number: `[EN]`
2. Include: Session ID, Turn, Timestamp, Context, Agent action, Interrupt type, User response
3. Extract pattern hypothesis
4. Check if it fits existing Pattern Candidates or creates new one
5. Update candidate evidence counts
6. If pattern reaches 3+ evidence with consistency, propose migration to CLAUDE.md

## Notes

- Not all interrupts indicate anti-patterns (some are legitimate context changes)
- Focus on patterns that caused user frustration or wasted effort
- Context matters: same action may be appropriate in different situations
