## Capability: infer-cli-command

CLI command `skill infer <source>` that runs the two-stage inference pipeline.

## Requirements

### Command Signature

```
skill infer <source> [flags]

Arguments:
  <source>    Local path or git URL to analyze

Flags:
  --install          Install the skill after inference
  --output <path>    Write skill.yaml to a specific path (default: stdout)
  --json             Output as JSON instead of YAML (non-interactive)
  --yes              Auto-accept LLM suggestions without confirmation
  --force            Re-infer even if skill.yaml already exists
  --git-ref <ref>    Git branch/tag/commit to use (for git URL sources)
```

### Flow

1. **Resolve source**: local path or git clone (reuse `aule-resolver` git clone logic)
2. **Check for existing `skill.yaml`**:
   - Found and no `--force` → print message "This source already has a skill.yaml. Use `skill install` directly." and exit 0
   - Found with `--force` → continue to inference
   - Not found → continue to inference
3. **Stage 1 (Discovery)**: run `scan_all` on repo root
   - Skills found → build `ManifestV2`, display summary, output
   - No skills found → print "No skills found in known locations." and continue to Stage 2
4. **Stage 2 (LLM Suggest)**: run signal gatherers, then LLM assessor
   - `can_infer: true` → display suggestions with confidence score
     - Interactive (default): show preview, ask "[Y]es / [n]o / [e]dit"
     - Non-interactive (`--yes` or `--json`): accept automatically
   - `can_infer: false` → print reasoning, exit 1
5. **Output**: write `skill.yaml` to `--output` path or stdout
6. **Install** (if `--install`): write `skill.yaml` into repo, hand off to `skill install` logic

### Display Format

**Stage 1 success:**
```
Scanning known skill locations...
  ✓ Found 2 skills in .claude/skills/
  ✓ Found 3 commands in .claude/commands/

Generated skill.yaml (v0.2.0):
  name: my-repo
  skills: 2 (foo, bar)
  commands: 3 (deploy, test, lint)
  adapters: claude-code
```

**Stage 2 — LLM suggestion:**
```
No skills found in known locations.

Analyzing repository for inferrable skills...
  Gathered: npm package, README (2.4k words), 3 bin scripts

LLM Assessment (confidence: 0.82):
  "This repo is a well-documented CLI tool with clear commands..."

Suggested skills:
  1. cool-tool — "Generates cool things from templates"
     permissions: [filesystem.read, filesystem.write]
     determinism: deterministic

Suggested tools:
  1. generate (node) — "Generate output from a template"

? Accept and generate skill.yaml? [Y/n/e]
```

**Stage 2 — LLM says no:**
```
No skills found in known locations.

Analyzing repository for inferrable skills...

This repo doesn't appear to contain skill-shaped content:
  "This is a data-only repository with CSV files and no documentation
   or executable tools that could be packaged as a skill."
```

### Error Cases

- Source path doesn't exist → error with message
- Git clone fails → error with message
- LLM unavailable and Stage 2 needed → error with `ANTHROPIC_API_KEY` hint
- Invalid `--output` path → error before running inference

### JSON Output

When `--json` is used, all output is structured JSON:

```json
{
  "stage": "discovery",
  "skills_found": 2,
  "manifest": { ... },
  "warnings": []
}
```

or

```json
{
  "stage": "suggest",
  "assessment": { "can_infer": true, "confidence": 0.82, ... },
  "manifest": { ... }
}
```

or

```json
{
  "error": "no_skills_found",
  "message": "This repo doesn't appear to contain skill-shaped content",
  "reasoning": "..."
}
```

## Acceptance Criteria

- `skill infer ./local-repo` with `.claude/skills/` present → outputs valid skill.yaml with discovered skills
- `skill infer ./local-repo` with no skill artifacts → calls LLM, shows suggestions interactively
- `skill infer https://github.com/user/repo` → clones, runs inference pipeline
- `skill infer ./repo --install` → infers then installs
- `skill infer ./repo --json` → outputs structured JSON, no interactive prompts
- `skill infer ./repo --yes` → auto-accepts LLM suggestions
- `skill infer ./repo` where `skill.yaml` exists → prints helpful message, exits 0
- `skill infer ./repo --force` where `skill.yaml` exists → re-infers
- `skill infer ./repo` with no API key and no known skills → clear error message
