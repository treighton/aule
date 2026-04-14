## Context

Today, `skill install` requires a `skill.yaml` at the source — whether that's a local path, git URL, or registry identifier. The resolver clones the repo, parses the manifest, and proceeds. If no manifest exists, the command fails with an error. This blocks adoption: most repos with useful skill-shaped content (`.claude/skills/`, CLI tools, MCP servers) can't participate in the ecosystem without manual manifest authoring.

The existing crates provide the building blocks:
- `aule-resolver` handles git cloning and local path resolution
- `aule-schema` defines `ManifestV2` and all v0.2.0 types
- `aule-adapter` generates runtime-specific output from a manifest
- `aule-cache` handles installation and activation

What's missing is the inference layer between "raw repo" and "valid manifest."

## Goals / Non-Goals

**Goals:**
- Deterministic extraction of skills from repos that already have skill artifacts in known locations
- LLM-assisted inference for repos without skill artifacts, with user confirmation
- Always produce valid v0.2.0 `ManifestV2` output
- CLI commands: `skill infer <source>` and `skill install <source> --infer`
- Clean separation: scanners → signals → LLM → manifest builder
- Reusable inference engine (crate) for future platform crawler integration

**Non-Goals:**
- Platform crawler / registry indexing (phase 2)
- Training or fine-tuning a model for inference (uses general-purpose Claude API)
- Inferring adapter-specific config (adapters section gets sensible defaults)
- Supporting v0.1.0 output (always v0.2.0)
- Auto-installing without user confirmation in LLM-suggest mode

## Decisions

### Decision 1: Two-stage pipeline — Discovery then Suggest

Stage 1 (Discovery) is deterministic and always runs. It scans known skill locations:
- `.claude/skills/**/*.md`
- `.codex/skills/**/*.md`
- `.claude/commands/**/*.md`
- `.codex/commands/**/*.md`
- `.claude/plugins/`, `plugin.json`
- Standalone `SKILL.md` files

If Stage 1 finds skills, it builds a `ManifestV2` mechanically — no LLM call, no API key needed. This is the fast path.

Stage 2 (LLM Suggest) only runs if Stage 1 finds nothing. It gathers repo signals (README, package metadata, file tree, executables), sends them to the LLM, and asks: "Can skills be inferred? If yes, what?" The LLM suggests, the user confirms.

**Rationale:** Repos with existing skill artifacts should be installable instantly without LLM overhead or API key requirements. The LLM is a fallback advisor, not the primary engine.

### Decision 2: Scanner output — `DiscoveredSkill`

```rust
pub struct DiscoveredSkill {
    pub name: String,
    pub description: Option<String>,
    pub entrypoint: PathBuf,
    pub commands: HashMap<String, PathBuf>,
    pub source_format: SourceFormat,
}

pub enum SourceFormat {
    ClaudeSkill,    // .claude/skills/
    CodexSkill,     // .codex/skills/
    ClaudeCommand,  // .claude/commands/ (command-only, no skill body)
    ClaudePlugin,   // plugin.json
    StandaloneSkillMd, // SKILL.md in repo root or subdirectory
}
```

Each scanner produces `Vec<DiscoveredSkill>`. The manifest builder merges them, deduplicating by name (preferring richer sources).

### Decision 3: Signal gatherer output — `InferredSignals`

```rust
pub struct InferredSignals {
    // From package metadata
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,

    // From repo analysis
    pub readme_content: Option<String>,
    pub file_tree: Vec<String>,
    pub language: Option<String>,
    pub runtime: Option<String>,
    pub runtime_version: Option<String>,
    pub executables: Vec<ExecutableInfo>,

    // From structured sources (if any)
    pub declared_inputs: Option<serde_json::Value>,
    pub declared_outputs: Option<serde_json::Value>,
    pub declared_permissions: Vec<String>,

    // Provenance
    pub signal_source: SignalSource,
}

pub struct ExecutableInfo {
    pub name: String,
    pub path: PathBuf,
    pub kind: ExecutableKind, // Binary, Script, EntryPoint
}

pub enum SignalSource {
    Npm,
    Python,
    Rust,
    Go,
    Generic,
}
```

Signal gatherers run in sequence: generic (always) → language-specific (if detected). Later gatherers enrich earlier signals.

### Decision 4: LLM integration — Claude API with structured output

The LLM assessor calls the Claude API with:
- System prompt defining the task and `ManifestV2` schema
- `InferredSignals` serialized as context
- README content (truncated to ~8k tokens if needed)
- Request for structured JSON response

The response schema:

```json
{
  "can_infer": true,
  "confidence": 0.85,
  "reasoning": "This repo contains a Node.js CLI tool with clear documentation...",
  "suggested_skills": [
    {
      "name": "cool-tool",
      "description": "Generates cool things from templates",
      "entrypoint_suggestion": "README.md",
      "permissions": ["filesystem.read", "filesystem.write"],
      "determinism": "deterministic",
      "inputs": { "type": "object", "properties": { "template": { "type": "string" } } },
      "outputs": { "type": "object", "properties": { "result": { "type": "string" } } }
    }
  ],
  "suggested_tools": [
    {
      "name": "generate",
      "description": "Generate output from a template",
      "using": "node",
      "entrypoint": "bin/generate.js"
    }
  ]
}
```

**API key:** Read from `ANTHROPIC_API_KEY` environment variable. If missing and Stage 2 is needed, error with a clear message: "No skills found in known locations. Set ANTHROPIC_API_KEY to enable LLM-assisted inference."

**Model:** Use `claude-sonnet-4-20250514` for cost/speed balance. Not configurable in v1 — can add `--model` flag later if needed.

### Decision 5: Manifest builder — merge and fill defaults

The builder takes either `Vec<DiscoveredSkill>` (Stage 1) or LLM suggestions (Stage 2) and produces a `ManifestV2`:

**From Stage 1 (extraction):**
- `schema_version`: `"0.2.0"`
- `name`: repo directory name or first skill name
- `files`: `["content/**"]`
- `skills`: mapped from `DiscoveredSkill` entries
  - `entrypoint`: the discovered file path
  - `description`: from YAML frontmatter or `"TODO"`
  - `version`: `"1.0.0"` (default)
  - `permissions`: `[]` (default — no LLM to infer)
  - `determinism`: `"probabilistic"` (safe default)
  - `commands`: from discovered command files
- `adapters`: `{ "claude-code": { "enabled": true } }` (default)

**From Stage 2 (LLM suggestion):**
- Same structure, but with LLM-provided descriptions, permissions, determinism, and I/O schemas
- Tools section populated if LLM detected executable tools
- Content files: if README.md is the only candidate, use it as entrypoint directly rather than generating synthetic content

### Decision 6: CLI command design

```
skill infer <source>              # Infer skill.yaml, preview and output
skill infer <source> --install    # Infer + install in one shot
skill infer <source> --output <path>  # Write skill.yaml to specific path
skill infer <source> --json       # Output inferred manifest as JSON (non-interactive)
skill install <source> --infer    # Install, using inference if no skill.yaml found
```

**`<source>`** accepts the same inputs as `skill install`: local path or git URL.

**Interactive behavior:**
- Stage 1 (extraction): non-interactive by default. Shows summary of discovered skills, outputs `skill.yaml`. Use `--confirm` to require approval.
- Stage 2 (LLM suggest): interactive by default. Shows LLM suggestions, asks for confirmation. Use `--yes` to auto-accept.

**`skill install <source> --infer` flow:**
1. Clone/resolve source
2. Check for `skill.yaml` → if found, normal install (ignore `--infer`)
3. If no `skill.yaml`, run infer pipeline
4. If Stage 1 produces results, auto-install (extraction is deterministic)
5. If Stage 2 is needed, show suggestions and ask for confirmation before installing

### Decision 7: Content file handling

When building the manifest, the inference engine needs to decide what `entrypoint` points to:

- **Existing `.claude/skills/foo/SKILL.md`**: use as-is, keep the original path
- **README.md as only candidate**: point `entrypoint` directly at `README.md` — don't generate synthetic content
- **LLM suggests a skill from a subdirectory's docs**: point at the most relevant markdown file

The manifest's `files` globs capture what to bundle. The inference engine adds globs based on what it found:
- Discovered in `.claude/skills/`: `files: [".claude/skills/**"]`
- README-based: `files: ["README.md"]`
- LLM detected tools in `bin/`: `files: ["README.md", "bin/**"]`

### Decision 8: Short-circuit on existing `skill.yaml`

If `skill infer` is run on a source that already has a `skill.yaml`:
- Print: "This source already has a skill.yaml. Use `skill install` directly."
- Exit with code 0 (not an error)
- Use `--force` to re-infer anyway (overwrite)

### Decision 9: Error handling for LLM unavailability

If Stage 2 is needed but the LLM is unavailable (no API key, network error, rate limit):
- Print: "No skills found in known locations."
- Print: "LLM-assisted inference unavailable: [reason]"
- Print hint: "Set ANTHROPIC_API_KEY to enable LLM inference, or add skill artifacts to the repo manually."
- Exit with code 1
- The `--json` flag wraps this in a structured error response

## Risks / Trade-offs

**Risk: LLM output quality.** The LLM may produce inaccurate permissions, wrong determinism classification, or miss skills. Mitigated by: always showing suggestions to the user for confirmation in interactive mode, and providing `--json` for programmatic consumers who want to post-process.

**Risk: API key requirement for Stage 2.** Users who don't have an Anthropic API key can't use LLM inference. Mitigated by: Stage 1 (extraction) works without any API key, and the error message is clear about what's needed and why.

**Risk: README quality varies wildly.** Some repos have excellent READMEs, others have none. The LLM may struggle with sparse information. Mitigated by: the LLM can respond with `"can_infer": false` and a clear explanation, rather than forcing bad output.

**Trade-off: Always v0.2.0 output.** Simple single-skill repos could use the lighter v0.1.0 format. We chose v0.2.0 for consistency and because v0.2.0 is a superset — a single-skill v0.2.0 manifest is only slightly more verbose than v0.1.0.

**Trade-off: New crate vs extending aule-schema.** We chose a new crate (`aule-infer`) because inference has different dependencies (HTTP client for LLM calls, format-specific parsers) and different consumers (CLI now, platform service later). Keeps `aule-schema` focused on parsing/validating known-good manifests.

**Trade-off: Direct Claude API vs pluggable LLM backend.** We chose direct Claude API for simplicity. Can abstract to a trait-based backend later if there's demand for local models or alternative providers.
