# Authoring Skills

This guide walks through creating, testing, and publishing a skill for Aulë.

## What Is a Skill?

A skill is a reusable capability for AI coding agents. It's a directory containing:

- **`skill.yaml`** — the manifest declaring identity, interface, and adapter configuration
- **`content/skill.md`** — the skill body that the agent sees and follows
- *(v0.2.0)* **`logic/`** — optional executable tool scripts and hook scripts

When you build a skill, Aulë generates runtime-specific output (e.g., `.claude/skills/my-skill/SKILL.md`) that the target agent can consume directly.

Aulë supports two manifest versions:
- **v0.1.0** — single-skill packages with prose content
- **v0.2.0** — multi-skill packages with executable tools, typed I/O, and lifecycle hooks

## Creating a Skill

### Scaffold

```bash
skill init --name my-skill
```

This creates:

```
my-skill/
├── skill.yaml
└── content/
    └── skill.md
```

### Write the manifest

Edit `skill.yaml`:

```yaml
schemaVersion: "0.1.0"
name: "my-skill"
description: "One line that explains when to use this skill"
version: "1.0"

content:
  skill: "content/skill.md"

contract:
  version: "1.0.0"
  inputs: "prompt"
  outputs: "prompt"
  permissions:
    - "filesystem.read"
  determinism: "probabilistic"

adapters:
  claude-code:
    enabled: true
  codex:
    enabled: true

metadata:
  author: "your-name"
  license: "MIT"
```

### Field reference (v0.1.0)

| Field | Required | Description |
|-------|----------|-------------|
| `schemaVersion` | Yes | Must be `"0.1.0"` |
| `name` | Yes | Alphanumeric + hyphens, unique identifier |
| `description` | Yes | One-line description — used for discovery and triggering |
| `version` | Yes | Semantic version of the skill |
| `content.skill` | Yes | Relative path to the skill body |
| `content.commands` | No | Map of command names to content files |
| `contract.version` | Yes | Semantic version of the contract (independent of skill version) |
| `contract.inputs` | Yes | `"prompt"` or a JSON Schema definition |
| `contract.outputs` | Yes | `"prompt"` or a JSON Schema definition |
| `contract.permissions` | Yes | List of required capabilities |
| `contract.determinism` | Yes | `deterministic`, `bounded`, or `probabilistic` |
| `contract.errors` | No | List of `{ code, description }` error definitions |
| `contract.behavior.timeout_ms` | No | Maximum execution time in milliseconds |
| `adapters` | Yes | Map of runtime targets with `enabled: true/false` |
| `metadata.author` | No | Author name or organization |
| `metadata.license` | No | License identifier (e.g., `MIT`, `Apache-2.0`) |
| `dependencies.tools` | No | External CLI tools the skill requires |
| `dependencies.skills` | No | Other skills this skill depends on |
| `identity` | No | Protocol-level identity (e.g., `skills.example.com/my-skill`) |
| `extensions` | No | Forward-compatible extension fields |

## v0.2.0 Multi-Skill Packages

v0.2.0 replaces `content` with `files` and `contract` with `skills`, and adds `tools` and `hooks`.

### v0.2.0 manifest example

```yaml
schemaVersion: "0.2.0"
name: "my-package"
description: "A multi-skill package with tools"
version: "1.0.0"

files:
  - "content/**"
  - "logic/**"

skills:
  analyzer:
    description: "Analyze code for issues"
    entrypoint: "content/analyzer.md"
    version: "1.0.0"
    permissions: ["filesystem.read"]
    determinism: "probabilistic"
    commands:
      analyze: "content/commands/analyze.md"

  fixer:
    description: "Auto-fix common issues"
    entrypoint: "content/fixer.md"
    version: "1.0.0"
    permissions: ["filesystem.read", "filesystem.write"]
    determinism: "bounded"

tools:
  scan:
    description: "Scan files for patterns"
    using: "node"
    version: ">= 18"
    entrypoint: "logic/tools/scan.ts"
    input:
      type: "object"
      properties:
        pattern: { type: "string" }
      required: ["pattern"]
    output:
      type: "object"
      properties:
        matches: { type: "array" }

hooks:
  onInstall: "logic/hooks/setup.sh"

adapters:
  claude-code:
    enabled: true
  codex:
    enabled: true

metadata:
  author: "your-name"
  license: "MIT"
```

### Key differences from v0.1.0

| v0.1.0 | v0.2.0 | Change |
|--------|--------|--------|
| `content.skill` | `skills.<name>.entrypoint` | Per-skill entrypoints (multiple skills per package) |
| `contract` | `skills.<name>.*` | Interface fields move into each skill definition |
| Single skill | `skills` map | Multiple skills with distinct interfaces |
| *(none)* | `files` | Glob patterns for bundled files |
| *(none)* | `tools` | Executable tools with typed JSON Schema I/O |
| *(none)* | `hooks` | Lifecycle scripts (onInstall, onActivate, onUninstall) |

### Tools

Tools are executable scripts that skills invoke via wrapper scripts. Each tool declares:

- **`using`** — runtime: `node`, `python`, or `shell`
- **`version`** — optional runtime version constraint
- **`entrypoint`** — path to the script
- **`input`/`output`** — JSON Schema for typed I/O

The adapter generates:
- A shell wrapper script per tool (`tools/<name>`) that agents can execute
- A `## Tools` documentation section appended to each SKILL.md

Tools accept JSON as the first positional argument and write JSON to stdout.

### Hooks

Lifecycle hooks run shell scripts at defined moments:

| Hook | When | Notes |
|------|------|-------|
| `onInstall` | After `skill install` | Use for dependency setup (e.g., `npm install`) |
| `onActivate` | After `skill activate` | Use for runtime verification (e.g., `node --version`) |
| `onUninstall` | Before `skill uninstall` | Use for cleanup |

Hook failure warns the user but does not roll back the operation.

### Migrating from v0.1.0

```bash
skill migrate --path my-skill/
```

This converts `content` → `files` + skill entrypoints, and `contract` → `skills` map. Review the output and adjust as needed.

### Write the skill body

Edit `content/skill.md`. This is what the AI agent sees. Write it as instructions:

```markdown
Analyze the current codebase for security vulnerabilities.

**Steps**

1. Scan for common vulnerability patterns:
   - SQL injection (string concatenation in queries)
   - XSS (unescaped user input in templates)
   - Command injection (user input in shell commands)

2. For each finding, report:
   - File and line number
   - Vulnerability type
   - Severity (critical, high, medium, low)
   - Suggested fix

3. Output a summary table at the end.

**Constraints**
- Only report confirmed patterns, not speculative issues
- Do not modify any files
```

Tips for writing effective skill content:

- **Be specific** — agents follow instructions literally
- **Use numbered steps** when order matters
- **State constraints explicitly** — what the agent should *not* do
- **Include examples** for complex output formats
- **Declare the stance** — is the agent an advisor, implementer, reviewer?

## Permissions

Skills must declare every capability they need. The permission vocabulary:

| Permission | What it allows |
|------------|----------------|
| `filesystem.read` | Reading files from disk |
| `filesystem.write` | Creating or modifying files |
| `process.spawn` | Running shell commands |
| `network.external` | Making outbound network requests |

A skill that only reads code and reports findings needs `filesystem.read`. A skill that runs tests needs `process.spawn`. A skill that calls an external API needs `network.external`.

Users can configure policies that block skills requiring permissions they haven't approved:

```json
{
  "policy": {
    "allow": ["@trusted-org/*"],
    "block": ["@unknown/*"]
  }
}
```

## Determinism Levels

| Level | Meaning | Example |
|-------|---------|---------|
| `deterministic` | Same input always produces same output | Schema validation, formatting |
| `bounded` | Output varies within defined bounds | Search with ranked results |
| `probabilistic` | Output is non-deterministic | LLM-driven analysis, creative tasks |

Most AI-driven skills are `probabilistic`. Use `deterministic` or `bounded` only when you can genuinely guarantee it.

## Commands

Skills can include named commands — entry points that users invoke explicitly (e.g., `/my-command` in Claude Code).

```yaml
content:
  skill: "content/skill.md"
  commands:
    run-analysis: "content/commands/run-analysis.md"
    show-report: "content/commands/show-report.md"
```

Each command is a separate Markdown file. Commands are generated alongside the skill during adapter output.

## Validating

```bash
skill validate --path my-skill/
```

This checks:

- Schema version is correct
- Name format is valid
- Version is valid semver
- Contract is well-formed
- Permissions are from the known vocabulary
- Adapter targets are recognized
- Content files exist at declared paths

Use `--json` for structured output in CI:

```bash
skill validate --path my-skill/ --json
```

## Building

```bash
# Build for a specific runtime
skill build --path my-skill/ --target claude-code --output .claude/skills/

# Build for all enabled targets (output goes to current directory)
skill build --path my-skill/
```

The generated file (`SKILL.md`) contains:

1. YAML frontmatter adapted for the target runtime
2. Your `content/skill.md` body, unchanged

Example output (`.claude/skills/my-skill/SKILL.md`):

```yaml
---
name: my-skill
description: One line that explains when to use this skill
license: MIT
metadata:
  author: your-name
  version: "1.0"
---

Analyze the current codebase for security vulnerabilities.
...
```

## Publishing

### Authenticate

```bash
skill login
```

This opens a browser for GitHub OAuth. Your GitHub username becomes your publisher identity.

### Publish

```bash
skill publish --path my-skill/
```

This registers your skill's git repository with the registry. The registry indexes the manifest metadata — skill content stays in your git repo.

### Versioning

Follow semantic versioning:

- **Patch** (1.0.0 → 1.0.1): Bug fixes, typo corrections
- **Minor** (1.0.0 → 1.1.0): New capabilities, backward-compatible changes
- **Major** (1.0.0 → 2.0.0): Breaking contract changes (different permissions, changed behavior)

The `contract.version` is independent of the skill `version`. Bump the contract version when the interface changes, even if the skill version stays the same.

## Testing Your Skill

### Manual testing

1. Build the skill for your preferred runtime
2. Copy the output to the runtime's skill directory
3. Use the skill in the agent and verify behavior

### Automated validation

Add your skill to the test suite by following the pattern in `crates/aule-adapter/tests/real_skills_test.rs`:

1. Generate adapter output for your skill
2. Compare against committed reference output
3. Assert byte-for-byte equality

This catches unintended changes to both your content and the generation pipeline.

## Example: Complete Skill

Here's a complete, minimal skill:

**`skill.yaml`:**

```yaml
schemaVersion: "0.1.0"
name: "code-reviewer"
description: "Review code changes for bugs, style issues, and security vulnerabilities"
version: "1.0"

content:
  skill: "content/skill.md"

contract:
  version: "1.0.0"
  inputs: "prompt"
  outputs: "prompt"
  permissions:
    - "filesystem.read"
    - "process.spawn"
  determinism: "probabilistic"

adapters:
  claude-code:
    enabled: true
  codex:
    enabled: true

metadata:
  author: "example"
  license: "MIT"
```

**`content/skill.md`:**

```markdown
Review the current code changes for quality and correctness.

**Process**

1. Run `git diff` to see staged and unstaged changes
2. For each changed file, analyze:
   - Logic errors or bugs
   - Security vulnerabilities (injection, XSS, etc.)
   - Style inconsistencies with the surrounding code
   - Missing error handling
3. Summarize findings by severity

**Output format**

For each issue:
- **File**: path and line number
- **Severity**: critical / high / medium / low
- **Issue**: what's wrong
- **Fix**: suggested correction

End with a summary: total issues by severity, and an overall assessment.

**Guidelines**
- Focus on substantive issues, not nitpicks
- Consider the context — don't flag patterns that are intentional
- If no issues found, say so clearly
```
