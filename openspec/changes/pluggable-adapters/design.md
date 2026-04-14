## Context

The adapter system currently lives in `aule-adapter` with two core types:
- `RuntimeTarget` — a struct with path templates and a `supports_commands` flag, constructed via hard-coded factory methods (`claude_code()`, `codex()`, `pi()`) and looked up via a match statement in `by_id()`
- `generate.rs` — the generation pipeline that builds frontmatter, concatenates content, writes files, and handles v0.1.0/v0.2.0 dispatch

The generation logic is already mostly generic. The only adapter-specific leakage is `PI_EXTRA_FIELDS` (a hardcoded constant for Pi frontmatter fields) and Claude Code-specific command frontmatter formatting. The manifest's `adapters: HashMap<String, AdapterConfig>` already accepts arbitrary string keys with an `extra: HashMap<String, Value>` for adapter-specific config — it's just that the generation pipeline ignores unknown keys.

## Goals / Non-Goals

**Goals:**
- Community can define and distribute adapters for new runtimes without modifying the Aulë source
- Two tiers: config-based (declarative, uses built-in pipeline) and script-based (full control via subprocess)
- Built-in adapters expressed as config-based definitions, eliminating special-casing
- Clear, versioned protocol for script communication
- CLI commands for adapter lifecycle management
- Adapter authors can provide validation scripts for pre-flight manifest checks

**Non-Goals:**
- WASM-based adapters (future consideration, not this change)
- Dependency management for script-based adapters (scripts handle their own deps)
- Custom command frontmatter schemas per adapter (defer to future work)
- Adapter marketplace or centralized registry (adapters are distributed as directories, not through a registry service)

## Decisions

### Decision 1: Two-tier adapter model

Config-based adapters use the existing generation pipeline with different parameters. Script-based adapters own the entire pipeline via stdin/stdout JSON.

**Rationale:** Most runtimes that follow the SKILL.md convention only differ in paths and frontmatter fields — config handles ~85% of cases with zero code. Script-based covers the remaining cases (runtimes with completely different output formats like Cursor's `.mdc` rules) without requiring Rust plugins or dynamic loading.

### Decision 2: Adapter definition schema (`adapter.yaml`)

```yaml
# Config-based
id: gemini
type: config
protocol: 1
description: "Adapter for Google Gemini CLI"
author: "community"

paths:
  skill: ".gemini/skills/{name}/SKILL.md"
  commands:
    path: ".gemini/commands/{namespace}/{command_name}.md"
    display_name: "{skill}: {command}"
    category: "Workflow"
    tags: ["workflow", "{skill}", "{command}"]

frontmatter:
  extra_fields:
    - model-preference
    - allowed-tools

validate: ./validate.sh   # optional
```

```yaml
# Script-based
id: cursor
type: script
protocol: 1
description: "Adapter for Cursor .mdc rules"
author: "cursor-community"

generate: ./generate.py
validate: ./validate.py    # optional
```

### Decision 3: Precedence order for adapter resolution

```
1. User-installed  (~/.skills/adapters/<id>/)   — highest
2. Skill-bundled   (<package>/adapters/<id>/)    — middle
3. Built-in        (compiled into binary)        — lowest
```

User-installed wins because the user knows their environment best. A skill author's bundled adapter may be outdated or wrong for a specific setup. Built-in adapters serve as sensible defaults.

### Decision 4: Built-in adapters become config definitions

The three built-in adapters are expressed internally as `AdapterDef::Config` values, compiled into the binary. This means:
- `PI_EXTRA_FIELDS` is eliminated — Pi's extras become `extra_fields: ["allowed-tools", "disable-model-invocation"]`
- All config-based adapters (built-in or external) go through the same code path
- Built-in adapters serve as test fixtures for the config system
- A user can override a built-in by installing a modified version at `~/.skills/adapters/claude-code/`

### Decision 5: Script protocol (v1)

**Input (stdin):**
```json
{
  "protocol_version": 1,
  "manifest": { /* full parsed manifest as JSON */ },
  "content": {
    "skills": {
      "my-skill": "raw skill.md content..."
    },
    "commands": {
      "my-skill": {
        "explore": "command body..."
      }
    },
    "files": {
      "logic/tools/generate.ts": "file content..."
    }
  },
  "adapter_config": {
    "enabled": true,
    "custom-field": "value"
  },
  "options": {
    "output_dir": "/path/or/null",
    "base_path": "/path/to/skill/package"
  }
}
```

**Output (stdout):**
```json
{
  "files": [
    {
      "relative_path": ".cursor/rules/my-skill.mdc",
      "content": "---\ndescription: ...\n---\n\nSkill body..."
    }
  ]
}
```

**Error (exit code != 0, stderr):**
```json
{
  "error": "Cursor adapter requires all skills to have descriptions",
  "details": [
    { "field": "skills.my-skill.description", "message": "missing required field" }
  ]
}
```

Content is provided fully resolved (file contents read, globs expanded) so scripts don't need filesystem access to the skill package.

### Decision 6: Validation script protocol

Same stdin as generate, different stdout:

```json
{
  "valid": true,
  "errors": [],
  "warnings": [
    { "field": "tools.lint", "message": "Shell tools may have limited support" }
  ]
}
```

Validation runs before generation. For config-based adapters, the built-in pipeline performs its own validation. For script-based adapters, the validation script (if present) is called first. If validation fails (errors present), generation is skipped.

### Decision 7: Protocol versioning

- `adapter.yaml` declares `protocol: N` (integer)
- CLI checks: if adapter protocol > CLI's max supported protocol → error with upgrade message
- If `protocol` is missing → assume 1
- Input JSON includes `protocol_version` so scripts can branch on it
- Backward-compatible additions (new optional fields) don't bump the protocol version; breaking changes do

### Decision 8: Adapter installation model

`skill adapters add` copies the adapter directory into `~/.skills/adapters/<id>/`. This is a simple file copy, not a symlink, so the adapter is self-contained. For git sources, the directory is cloned and the `.git` directory removed.

`skill adapters remove <id>` deletes the directory. Built-in adapters cannot be removed.

### Decision 9: `skill adapters test` behavior

Tests an adapter by running it against a synthetic or user-provided skill manifest:

1. If adapter has a `validate` script → run validation and report results
2. Run generation against a minimal test manifest (or `--path ./my-skill/` for a real one)
3. Verify output: files are valid UTF-8, paths are within expected bounds, no path traversal
4. For config-based: verify path templates resolve correctly, frontmatter is valid YAML
5. For script-based: verify script is executable, returns valid JSON, exits 0

## Risks / Trade-offs

**Risk: Script adapter security.** Script adapters execute arbitrary code. Mitigated by: adapters are explicitly installed by the user (not auto-discovered from dependencies), and the `test` command helps verify behavior before use. This is the same trust model as npm/pip scripts.

**Risk: Protocol evolution.** Adding fields to the JSON protocol could break scripts that do strict parsing. Mitigated by: documenting that scripts SHOULD ignore unknown fields, and using the protocol version for breaking changes only.

**Trade-off: File copy vs symlink for installation.** Copying is simpler and self-contained but means updates require re-installation. Acceptable for now; could add `skill adapters update` later.

**Trade-off: Giving scripts resolved content vs paths.** We chose resolved content so scripts don't need filesystem access. This means large skills send more data over stdin, but keeps scripts pure functions of their input.
