---
name: git-workflow
description: Git workflow standards - branch management, commit conventions, and PR creation process. Use when planning work that involves git operations or PR submissions.
---

# Git Workflow and Conventions

Worktree-aware Git workflow using `mise run git:*` tasks.

## Quick Reference

```sh
# Basic operations
mise run git:status            # Show current state + next action
mise run git:home              # Switch to home branch + sync with origin/main
mise run git:new <branch>      # Create new branch from origin/main
mise run git:cleanup [branch]  # Delete merged branch + return to home

# PR lifecycle (CI wait -> browser open -> merge watch -> cleanup)
mise run git:open-pr <pr#>     # All-in-one: CI -> open -> watch -> cleanup

# Pause / discard / undo
mise run git:pause [message]   # WIP commit + return to home (for switching tasks)
mise run git:abandon           # Discard all changes + return to home
mise run git:undo              # Soft reset HEAD~1 (undo last commit)

# Stacked PRs
mise run git:sync              # Sync current branch after base PR merge
```

> **Note**: These tasks use the `gw` Rust CLI ([crates.io](https://crates.io/crates/git-workflow)). Install with `cargo install git-workflow`.

> **Pitfalls**
> - **Do not run `git checkout main`** — use `mise run git:home` instead (worktree conflict)
> - **Do not use `git stash`** — use `mise run git:pause` instead (creates WIP commit for safer worktree switching)
> - **Do not manually rebase stacked PRs** — use `mise run git:sync` instead (updates GitHub PR base + rebases)

## Standard Workflow: Code -> PR

**Every code change should become a PR.** Follow this flow:

```
1. Branch   -> mise run git:new -- feature/your-feature
2. Code     -> make changes
3. Commit   -> git add -A && git commit -m "feat: ..."
4. Push     -> git push -u origin feature/your-feature
5. PR       -> gh pr create -a "@me" -t "feat: ..."
6. Open     -> mise run git:open-pr <pr#>  (CI wait -> browser -> merge watch -> cleanup)
               ⚠ Run ONCE after final push. If CI fails, stop watcher -> fix -> push -> relaunch.
7. Cleanup  -> (auto: merge detected -> git:cleanup runs)
```

### git:open-pr — PR Lifecycle Management (Claude Code Background Task)

After creating a PR, run in background:

```
Claude: [Bash(run_in_background=true)] mise run git:open-pr -- <pr#>
```

3 phases run automatically:
1. **CI wait** — `gh pr checks --watch` waits for CI to pass
2. **Open in browser** — Uses `scripts/open-url.local.sh` if present (gitignored), else default browser
3. **Merge watch** — Polls PR state every 30s
   - MERGED -> macOS notification + `mise run git:cleanup` -> exit
   - CLOSED -> message -> exit

**Singleton rule — only ONE watcher per PR:**
- `git:open-pr` must run **only once** per PR — after the final push, when no more changes are expected.
- If CI fails and you need to push a fix: **stop the existing watcher** with `TaskStop` first, fix and push, then launch a new `git:open-pr`.
- Never have multiple `git:open-pr` background tasks running for the same PR.
- Use `--no-wait` to skip CI wait phase when you just want the merge watcher.

**Claude behavior**: When background task output arrives via `<system-reminder>`, Claude MUST:
1. Read the output file with `TaskOutput` or `Read`
2. Report the result to the user immediately
3. Show key information: merged/closed status, cleanup success/failure

**Run `mise run git:status` at any point to see what to do next.**

### If you have uncommitted changes on home branch

This happens when you made changes before creating a branch. Fix it:

```sh
# 1. Create branch (keeps your changes)
mise run git:new -- feature/your-feature

# 2. Now follow git:status
mise run git:status
# -> Will suggest: commit, push, create PR
```

## Proactive Workflow

**Always run `mise run git:status` and follow the "Next:" action.**

The status command automatically detects:
- Working directory state (clean/uncommitted changes)
- Sync state with upstream (pushed/unpushed/behind)
- PR state (none/open/merged/closed)

And suggests the appropriate next action:

| Status Output | Action |
|--------------|--------|
| `Next: start new work` | `mise run git:new -- feature/...` |
| `Next: commit changes` | `git add -A && git commit -m "..."` |
| `Next: push to remote` | `git push -u origin <branch>` |
| `Next: create pull request` | `gh pr create -a "@me" -t "..."` |
| `Waiting: PR #N in review` | Wait for CI/review, or start parallel work |
| `Next: cleanup merged branch` | `mise run git:cleanup` |
| `Next: rebase on latest main` | `git fetch && git rebase origin/main` |
| `Next: sync (base 'X' was merged)` | `mise run git:sync` |

## Branch Naming Convention

- `feature/` - New features
- `fix/` - Bug fixes
- `chore/` - Maintenance tasks
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions or fixes

## Commit Message Format

Use Conventional Commits (one-line only):

```
<type>[optional scope]: <description>
```

### Types

- **feat**: New feature
- **fix**: Bug fix
- **chore**: Maintenance tasks
- **docs**: Documentation changes
- **refactor**: Code refactoring
- **perf**: Performance improvements
- **test**: Adding or modifying tests

### Examples

```
feat(providers): add Gemini log parser
fix: correct token count in session summary
chore: bump version to 0.8.0
```

## PR Standards

- **Language**: English
- **Title**: Descriptive. Use commit message if single commit
- **Body**: Explain changes and context
- **Size**: Keep PRs small and focused

## About Worktrees

This project uses git worktree. Each worktree has a **home branch**.

- Home branch = directory name (e.g., `agtrace-2/` -> `agtrace-2` branch)
- Never checkout `main` directly (used by another worktree)
- Always branch from `origin/main`

The `mise run git:*` tasks handle this automatically.

## Workflow: Stacked PRs

When working on features that depend on unmerged work:

### Creating Stacked PRs

```sh
# 1. Create base PR (depends on main)
mise run git:new -- feature/base
# ... commit, push, create PR ...

# 2. Create child PR (depends on base)
git checkout feature/base
git checkout -b feature/child
# ... commit, push ...
gh pr create --base feature/base -t "feat: child feature"
```

### After Base is Merged

When the base PR is merged, `gw status` will detect this and suggest syncing:

```
Base PR: #123 [MERGED]
-> Next: sync (base 'feature/base' was merged)

  mise run git:sync
```

Run `mise run git:sync` to:
1. Update child PR's base to `main`
2. Rebase child branch on `origin/main`
3. Force push the updated branch

**Do not manually `git rebase`** — always use `git:sync` to keep GitHub PR base and local branch in sync.

## Workflow: Incidental Refactoring (Yak Shaving Protocol)

When you discover necessary refactoring during feature work, **don't mix it into the feature branch**.

### The Flow

1. **Pause current work** (WIP commit + return to home):
   ```sh
   mise run git:pause -- "waiting for refactor Y"
   # Creates WIP commit, comments on PR if exists, switches to home
   ```

2. **Create & ship the refactor**:
   ```sh
   mise run git:new -- refactor/descriptive-name
   # ... implement the fix ...
   mise run git:status  # Follow the "Next:" action
   # ... commit, push, PR, merge, cleanup ...
   ```

3. **Resume feature work**:
   ```sh
   git checkout feature/original-branch
   mise run git:undo  # Undo the WIP commit (changes return to staged area)
   mise run git:status  # Continue from where you left off
   ```
