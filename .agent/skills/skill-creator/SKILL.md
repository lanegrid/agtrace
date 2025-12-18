---
name: skill-creator
description: Skill Creator - Meta-skill for creating new domain-specific skills. Automatically activates when creating skills about specific topics or expertise areas.
---

# Skill Creator - Meta-Skill

You are an expert at creating domain-specific skills for Claude Code. This meta-skill helps you create new skills that capture specialized knowledge about specific parts of the codebase, tools, or workflows.

## Skill Structure

Every skill follows this structure:

```
.agent/skills/<skill-name>/
└── SKILL.md
```

The `.claude/skills` directory is a symlink to `.agent/skills`:

```
.agent/skills/           ← source of truth (actual files)
.claude/skills -> ../.agent/skills  ← symlink for Claude Code access
```

This means you only need to create files in `.agent/skills/` - they are automatically accessible via `.claude/skills/`.

## Skill File Format

A skill MUST have the following frontmatter and structure:

```markdown
---
name: <skill-name>
description: <One-line description of what this skill does and when it activates>
---

# <Skill Title>

<Introduction paragraph explaining what this skill is for>

## <Section 1: Core Knowledge>

<Content that the skill needs to know>

## <Section 2: Patterns/Architecture/etc.>

<More specialized content>

## When to Use This Skill

This skill should be activated when:
- <Specific trigger condition 1>
- <Specific trigger condition 2>
- <etc.>

## <Additional Sections as Needed>

<More content specific to the domain>
```

## Frontmatter Requirements

The frontmatter MUST include:
- **name**: Kebab-case skill name (e.g., `agtrace-cli-expert`, `skill-creator`)
- **description**: One-line description that includes:
  - What the skill is about
  - When it should be activated
  - Often includes "(project)" suffix for project-specific skills

## Good Skill Characteristics

### 1. Domain-Specific Knowledge
Skills should capture specialized knowledge about:
- Specific subsystems (e.g., CLI, database layer, parser)
- Tools or workflows (e.g., ExecPlans, testing patterns)
- Architecture patterns (e.g., handler pattern, view abstraction)

### 2. Self-Contained
Each skill should:
- Be understandable without reading other skills
- Include all necessary context
- Reference specific file paths when relevant
- Define any technical terms used

### 3. Actionable
Skills should help with:
- Understanding code structure
- Making modifications
- Following established patterns
- Debugging common issues

### 4. Well-Organized
Use clear sections:
- Overview/Introduction
- Architecture/Structure
- Patterns and conventions
- File organization
- Common tasks or use cases
- When to activate this skill

## Skill Creation Process

### Step 1: Research the Domain

Before creating a skill, thoroughly research:
1. Read relevant source code
2. Understand the architecture
3. Identify key patterns
4. Note important file paths
5. Understand the problem domain

Use the Task tool with `subagent_type=Explore` for comprehensive codebase exploration.

### Step 2: Structure the Knowledge

Organize information into logical sections:
- Start with high-level overview
- Progress to detailed patterns
- Include practical examples
- End with usage guidance

### Step 3: Create the Skill File

```bash
# Create directory
mkdir -p .agent/skills/<skill-name>

# Write SKILL.md with frontmatter and content
# (Use Write tool)
```

### Step 4: Verify

```bash
# Verify the skill file is readable
cat .agent/skills/<skill-name>/SKILL.md

# Verify it's accessible via .claude/skills (through symlink)
ls -la .claude/skills/<skill-name>/
```

Note: No need to create individual symlinks! The `.claude/skills` directory is already a symlink to `.agent/skills`.

## Types of Skills to Create

### 1. Subsystem Expert Skills
Deep knowledge of a specific subsystem:
- Example: `agtrace-cli-expert` (CLI structure and patterns)
- Example: `database-layer-expert` (database schema and queries)
- Example: `parser-expert` (log parsing and schema-on-read)

### 2. Workflow Skills
Capturing specific workflows:
- Example: `execplan` (creating execution plans)
- Example: `testing-workflow` (testing patterns and practices)
- Example: `release-process` (release and deployment)

### 3. Tool-Specific Skills
Knowledge about specific tools:
- Example: `rust-best-practices` (Rust patterns in this project)
- Example: `git-workflow` (Git conventions and commit style)
- Example: `ci-cd-expert` (CI/CD pipeline knowledge)

### 4. Meta Skills
Skills about skills:
- Example: `skill-creator` (this skill!)
- Example: `documentation-generator` (creating docs)

## Skill Naming Conventions

Follow these conventions:
- Use kebab-case (lowercase with hyphens)
- Be descriptive but concise
- Include domain or tool name
- Often end with `-expert`, `-workflow`, or similar suffix

Good examples:
- `agtrace-cli-expert`
- `execplan`
- `skill-creator`
- `database-layer-expert`

Bad examples:
- `CLI` (too short, not descriptive)
- `the_rust_expert` (use kebab-case)
- `everything-about-parsers-and-schemas` (too long)

## Example: Creating a New Skill

Here's a complete example of creating a database expert skill:

```bash
# 1. Research phase (use Task tool with Explore agent)
# Explore database-related files, schemas, queries

# 2. Create directory
mkdir -p .agent/skills/database-expert

# 3. Write SKILL.md (use Write tool)
cat > .agent/skills/database-expert/SKILL.md << 'EOF'
---
name: database-expert
description: Database Expert - Deep knowledge of agtrace database schema, queries, and indexing strategy. Activates for database-related tasks.
---

# Database Expert

[Content about database structure, schema, common queries, etc.]
EOF

# 4. Verify - it's automatically available via .claude/skills
ls -la .claude/skills/database-expert
cat .agent/skills/database-expert/SKILL.md
```

That's it! No symlink creation needed - the skill is automatically accessible.

## Best Practices

### DO:
- Research thoroughly before writing
- Include specific file paths and line numbers when relevant
- Provide concrete examples
- Explain architectural patterns
- Define technical terms
- Include "When to Use This Skill" section
- Keep content focused on one domain

### DON'T:
- Create skills that are too broad
- Duplicate information across skills
- Include outdated information
- Use vague descriptions
- Forget the frontmatter
- Create skills without proper research

## Updating Existing Skills

Skills are living documents. Update them when:
- Code structure changes
- New patterns are established
- Better examples are found
- Architecture evolves

To update:
1. Read the existing skill file
2. Make targeted edits
3. Ensure all sections remain accurate
4. Update any file path references

## Integration with Claude Code

Skills are automatically discovered by Claude Code when:
- They are in `.claude/skills/` directory
- They have proper frontmatter
- They follow the SKILL.md naming convention

Claude will use skills:
- Based on the description in frontmatter
- When user asks related questions
- When working on related code

## When to Use This Meta-Skill

Use `skill-creator` when:
- User asks to create a new skill about a topic
- You need to capture specialized knowledge
- You want to document a subsystem or workflow
- Creating documentation for future development
- User explicitly mentions creating a skill or expertise area

## Common Patterns

### Pattern 1: Subsystem Deep-Dive

Research → Structure (Overview, Architecture, Patterns, Files, Tasks) → Write → Verify

### Pattern 2: Workflow Documentation

Identify workflow → Document steps → Provide examples → Explain when to use → Write → Verify

### Pattern 3: Reference Documentation

Gather facts → Organize by category → Add usage guidance → Write → Verify

## Troubleshooting

### Skill Not Appearing

Check:
1. Frontmatter is properly formatted
2. File exists in `.agent/skills/<skill-name>/SKILL.md`
3. File is named `SKILL.md` (case-sensitive)
4. `.claude/skills` symlink points to `.agent/skills` correctly

### Skill Not Activating

Check:
1. Description is clear about when to activate
2. Skill content is relevant to the task
3. Frontmatter description matches the content

## Summary

This meta-skill enables creation of domain-specific expertise that Claude Code can leverage. By following the structure and best practices outlined here, you can create effective skills that capture specialized knowledge and improve the development experience.

Key steps:
1. Research thoroughly
2. Structure logically
3. Write with frontmatter to `.agent/skills/<skill-name>/SKILL.md`
4. Verify and test

Remember:
- Skills are living documents. Keep them updated and focused on their specific domain.
- No symlinks needed! `.claude/skills` is already a symlink to `.agent/skills`.
- Just create files in `.agent/skills/` and they're automatically accessible.
