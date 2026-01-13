---
name: git-workflow
description: Git workflow standards - branch management, commit conventions, and PR creation process. Use when planning work that involves git operations or PR submissions.
---

# Git Workflow and Conventions

Standard Git workflow including branching strategy, commit message format, and pull request creation.

## Quick Start Workflow

```sh
# Step 1: Setup
git fetch --prune

# Step 2: Create feature branch
git switch -c feature/your-branch-name origin/main

# Step 3: Make changes and commit
git add [<files>]
git commit -m "[commit message]"
git push --set-upstream origin feature/your-branch-name

# Step 4: Create Pull Request
gh pr create -a "@me" -t "[title]"

# Step 5: Check CI Status
gh pr checks --watch

# Step 6: Open PR in browser for review
gh pr view --web

# Step 7: After merging, clean up branches
# Standard:
git switch main && git pull
git branch -d <branch_name>
# Worktree (when main is used elsewhere):
git switch <worktree-branch> && git pull origin main
git branch -d <branch_name>
```

## Branching Strategy

### Always Branch from `origin/main`

Create all feature branches from the latest `origin/main`:

```sh
git fetch --prune
git switch -c feature/your-branch-name origin/main
```

### Git Worktree Support

When working in a git worktree (e.g., `wt-2` directory), the `main` branch is used by another worktree and cannot be checked out directly. In this case:

1. **The directory name (e.g., `wt-2`) acts as the local main branch equivalent**
2. **Always branch from `origin/main`** (not local main)
3. **After merge, update with**: `git pull origin main` (instead of `git switch main && git pull`)

```sh
# In worktree environment - post-merge cleanup:
git switch <worktree-branch>  # e.g., wt-2
git pull origin main
git branch -d <merged-branch-name>
```

### Branch Naming Convention

- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions or fixes

## Commit Message Format

Use Conventional Commits format:

```
<type>[optional scope]: <description>

[optional body]

[optional footer]
```

### Commit Types

- **fix**: Patches a bug in your codebase
- **feat**: Introduces a new feature to the codebase
- **chore**: Maintenance tasks (dependencies, configs, etc.)
- **docs**: Documentation changes
- **style**: Code style changes (formatting, white-space, etc.)
- **refactor**: Code refactoring without changing functionality
- **perf**: Performance improvements
- **test**: Adding or modifying tests

### Commit Examples

**No body**:

```
docs: correct spelling of CHANGELOG
```

**With scope**:

```
feat(api): add user authentication endpoint
```

**With body and footer**:

```
fix: correct minor typos in code

see the issue for details on the typos fixed

closes issue #12
```

## Pull Request Creation

### PR Standards

- **Language**: English
- **Title**: Descriptive of changes. Use commit message if branch has 1 commit
- **Body**: Explain changes, why they were made, and any relevant context
- **Keep PRs small and focused**: Easy to review and test
- **Keep PR Body up to date**: Reflect current state of changes

### Create PR with GitHub CLI

```sh
# Basic PR creation
gh pr create -a "@me" -t "[title]"

# Wait for CI checks to complete
gh pr checks --watch

# Open in browser for review
gh pr view --web
```

## Post-Merge Cleanup

After PR is merged:

```sh
# Standard environment:
git switch main
git pull
git branch -d <merged-branch-name>

# Git worktree environment (when main is used by another worktree):
git switch <worktree-branch>  # e.g., wt-2
git pull origin main
git branch -d <merged-branch-name>
```

## Workflow: Incidental Refactoring (Yak Shaving Protocol)

When you identify necessary refactoring while working on a feature branch (but it's outside the scope or too large):

**Do not mix refactoring into the feature branch.** Use a Draft PR as your "base camp" to avoid getting lost in Yak Shaving.

### Why Draft PR as Base Camp?

- **Outsource memory to GitHub**: Free your brain's stack by writing down what you're doing
- **Always have a way back**: No matter how deep the refactoring rabbit hole, your Draft PR anchors you
- **Terminal-first**: Use `gh` commands to stay focused without browser context-switching

### The Flow

1.  **Anchor the Context (Don't forget why!)**:
    Before leaving, ensure a Draft PR exists for your current feature and document what you were doing.
    ```sh
    # If PR doesn't exist yet, create a Draft PR as a notepad
    gh pr create --draft -a "@me" -t "WIP: [Current Feature]" -b "- [ ] Implement X (Paused for refactoring Y)"

    # If PR exists, add a comment about the distraction
    gh pr comment --body "Paused to refactor [Component Y]. Will resume after merging that."
    ```

2.  **Stash current changes**:
    ```sh
    git stash -u -m "Paused: [Current Feature] - waiting for refactor"
    ```

3.  **Create & Ship Refactor** (works in both standard and worktree environments):
    ```sh
    git fetch --prune
    git switch -c refactor/descriptive-name origin/main
    # ... Implementation ...
    git push --set-upstream origin refactor/descriptive-name
    gh pr create -a "@me" -t "refactor: [description]"
    ```

4.  **Resume Feature Work**:
    ```sh
    git switch feature/original-branch
    # Option A: Rebase onto the refactor branch to use it immediately
    git rebase refactor/descriptive-name
    # Option B: Wait for merge, then rebase on main
    # git fetch --prune && git rebase origin/main
    git stash pop
    ```

5.  **Review the Anchor**:
    Check your Draft PR to remember the original goal.
    ```sh
    gh pr view --web
    ```
