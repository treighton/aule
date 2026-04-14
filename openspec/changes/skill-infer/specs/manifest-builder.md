## Capability: manifest-builder

Assembles a complete, valid `ManifestV2` from either extracted skills (Stage 1) or LLM suggestions (Stage 2).

## Requirements

### From Stage 1 (Extraction)

- Input: `Vec<DiscoveredSkill>` + repo root path
- Map each `DiscoveredSkill` to a `SkillDefinition`:
  - `entrypoint`: the discovered file path (relative to repo root)
  - `description`: from YAML frontmatter, or `"TODO: add description"` if missing
  - `version`: `"1.0.0"` (default)
  - `permissions`: empty vec (no LLM to infer)
  - `determinism`: `"probabilistic"` (safe default)
  - `commands`: from discovered command files, mapped by command name
- `name`: repo directory name (last path segment) or package name if available
- `schema_version`: `"0.2.0"`
- `files`: derive from discovered paths — e.g., skills in `.claude/skills/` → `[".claude/skills/**"]`
- `adapters`: `{ "claude-code": { "enabled": true } }` (default)
- No `tools` or `hooks` (Stage 1 doesn't detect these)

### From Stage 2 (LLM Suggestions)

- Input: `LlmAssessment` + `InferredSignals` + repo root path
- Map each `SuggestedSkill` to a `SkillDefinition`:
  - `entrypoint`: from `entrypoint_suggestion`
  - `description`: from LLM
  - `version`: from `InferredSignals.version` or `"1.0.0"`
  - `permissions`: from LLM
  - `determinism`: from LLM
  - `inputs`/`outputs`: from LLM (if provided)
  - `commands`: empty (LLM doesn't currently suggest commands)
- Map each `SuggestedTool` to a `Tool`:
  - `using`: from LLM
  - `entrypoint`: from LLM
  - `version`: from LLM (if provided)
- `name`: from `InferredSignals.name` or repo directory name
- `files`: derive from suggested entrypoints and tools — build minimal glob set
- `adapters`: `{ "claude-code": { "enabled": true } }` (default)
- `metadata`: populate `author` and `license` from `InferredSignals` if available

### Serialization

- Output valid YAML matching the v0.2.0 manifest schema
- Use `serde_yaml` for serialization
- Preserve field ordering: `schemaVersion`, `name`, `description`, `version`, `files`, `skills`, `tools`, `hooks`, `adapters`, `metadata`
- Omit optional fields that are empty/None (don't emit `tools: {}` or `hooks: null`)

### Validation

- After building, validate the produced `ManifestV2` using `aule-schema`'s existing parser
- If validation fails, return an error with details — don't produce an invalid manifest
- Validate that all referenced file paths exist in the repo

## Acceptance Criteria

- Given 2 discovered skills and 3 commands from Stage 1, builder produces a valid `ManifestV2` with 2 skill entries and appropriate commands
- Given an LLM assessment with 1 skill and 2 tools, builder produces a valid `ManifestV2` with skills and tools sections
- Produced YAML round-trips through `aule-schema` manifest parser without errors
- File paths in the manifest are all relative to repo root
- Optional sections (tools, hooks, metadata) are omitted when empty
