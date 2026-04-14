## Why

There is no way to use a repo as a skill unless the author has already written a `skill.yaml`. This means the Aulë ecosystem can only grow as fast as authors adopt the manifest format — a chicken-and-egg problem. Many repos already contain skill-shaped content (`.claude/skills/`, `.codex/commands/`, README-based guides, CLI tools, MCP servers) but can't be installed or discovered through the skill toolchain. For the future hosted platform, we also need the ability to index GitHub repos that don't ship a manifest, but that is phase 2 — this change focuses on the CLI-local workflow.

## What Changes

- Add a new `aule-infer` crate that implements a two-stage skill inference pipeline:
  - **Stage 1 (Discovery)**: Deterministic scanners check known skill locations (`.claude/skills/`, `.codex/skills/`, `.claude/commands/`, `.codex/commands/`, `plugin.json`, standalone `SKILL.md`) and extract existing skills into a `ManifestV2`
  - **Stage 2 (LLM Suggest)**: If no skills are found in known locations, gather repo signals (README, package metadata, file tree, executables) and ask an LLM whether skills can be inferred — the LLM suggests, the user decides
- Output is always a valid v0.2.0 `ManifestV2`
- Add CLI command `skill infer <source>` with `--install` flag to chain into installation
- Add `--infer` flag to existing `skill install` command to trigger inference when no `skill.yaml` is found
- Interactive mode (default for Stage 2 / LLM suggestions): preview + confirm before writing
- Non-interactive mode (default for Stage 1 / extraction): generates and outputs directly

## Capabilities

### New Capabilities
- `skill-scanner`: Deterministic scanners for known skill locations — `.claude/skills/`, `.codex/skills/`, `.claude/commands/`, `.codex/commands/`, `plugin.json`, standalone `SKILL.md` files. Extracts skill name, description, entrypoint, and commands from YAML frontmatter and file structure.
- `signal-gatherer`: Collects repo metadata for LLM assessment — README content, package metadata (npm/python/rust/go), file tree, executable scripts, language detection, runtime detection. Produces a structured `InferredSignals` bundle.
- `llm-assessor`: Takes `InferredSignals` and asks the LLM two questions: (1) can skills be inferred from this repo? (2) if yes, what skills, with what descriptions, permissions, and contract? Returns structured JSON that maps to `ManifestV2` fields.
- `manifest-builder`: Assembles a complete `ManifestV2` from either extracted skills (Stage 1) or LLM suggestions (Stage 2). Fills defaults for fields that can't be determined.
- `infer-cli-command`: `skill infer <source>` command — accepts local path or git URL, runs the two-stage pipeline, previews the result, optionally installs.
- `install-infer-flag`: `--infer` flag on `skill install` — if the resolved source has no `skill.yaml`, falls back to the infer pipeline before installing.

### Modified Capabilities
- `install-cli-command`: Extended with `--infer` flag that triggers inference pipeline when no manifest is found at the source.

## Impact

- **New crate** (`aule-infer`): Core inference logic — scanners, signal gatherers, LLM assessor, manifest builder. Dependencies: `aule-schema` (for `ManifestV2` types), `reqwest` (for LLM API calls), `serde_json`.
- **CLI** (`aule-cli`): New `skill infer` subcommand, `--infer` flag on `skill install`, new dependency on `aule-infer`.
- **Resolver** (`aule-resolver`): No changes — `skill infer` reuses the existing git clone logic but handles the "no skill.yaml" case instead of erroring.
- **Schema** (`aule-schema`): No changes — `ManifestV2` already has all needed types.
- **Adapter** (`aule-adapter`): No changes — once `skill infer` produces a `ManifestV2`, the existing adapter pipeline handles generation.
- **Cache** (`aule-cache`): No changes — installation flow is unchanged once a manifest exists.
