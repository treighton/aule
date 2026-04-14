# Architecture

This document describes Aulë's internal architecture for contributors and anyone integrating with the protocol.

## Overview

Aulë is a Cargo workspace with six crates organized as a library-first design. The CLI binary (`skill`) is a thin wrapper — all logic lives in the library crates.

```
aule-cli (binary)
  ├── aule-schema      Protocol types, parsing, validation
  ├── aule-adapter     Runtime adapter generation
  ├── aule-resolver    Multi-source version resolution
  ├── aule-cache       Local artifact storage and activation
  └── aule-infer       Skill inference (discovery + LLM suggestion)
```

## Crate Details

### aule-schema

The foundation. Defines all protocol types and validation logic.

**Key types:**

- `Manifest` — v0.1.0 parsed representation of `skill.yaml`
- `ManifestV2` — v0.2.0 parsed representation (multi-skill, tools, hooks)
- `ManifestAny` — version-dispatched enum: `V1(Manifest)` | `V2(ManifestV2)`
- `SkillDefinition` — per-skill interface in v0.2.0 (entrypoint, permissions, determinism, I/O)
- `Tool` — executable tool declaration (runtime, entrypoint, typed input/output)
- `Hooks` — lifecycle scripts (onInstall, onActivate, onUninstall)
- `Contract` — versioned interface declaration (v0.1.0 only)
- `Permission` — capability requirement with scope (e.g., `filesystem.read`, `process.spawn`)
- `RequestEnvelope` / `ResponseEnvelope` — invocation protocol for structured skill calls

**Validation pipeline:**

```
Raw YAML string
  → parse_manifest_any()             two-phase: peek schemaVersion, then deserialize
  ├── v0.1.0:
  │     → serde_yaml::from_str::<Manifest>()
  │     → validate_manifest()        name, version, contract, content paths
  │     → validate_contract()        permissions, inputs/outputs, determinism
  └── v0.2.0:
        → reject if 'content' or 'contract' present
        → serde_yaml::from_str::<ManifestV2>()
        → validate_manifest_v2()     skills, tools, hooks, files, permissions
  → Ok(ManifestAny)
```

**Design decisions:**

- Permissions use a flat `scope.action` vocabulary rather than hierarchical namespaces. This keeps policy matching simple and avoids ambiguity.
- The envelope types (`RequestEnvelope`, `ResponseEnvelope`) support both prompt-based and JSON Schema-based contracts. Most skills use `"prompt"` — structured contracts are for machine-to-machine skill composition.
- Schema version routing uses two-phase parsing (peek at `schemaVersion` from a `serde_yaml::Value`, then deserialize the correct struct) for precise error messages rather than serde's untagged enum approach.

### aule-adapter

Pluggable adapter system that generates runtime-specific output files from a manifest and skill content. Adapters can be built-in, user-installed, or bundled with a skill package.

**Key types:**

- `AdapterDef` — enum defining an adapter: `Config(ConfigAdapter)` (declarative) or `Script(ScriptAdapter)` (external script)
- `AdapterRegistry` — discovers and resolves adapters from multiple sources with precedence
- `GeneratedFile` — a path + content pair produced by the generator

**Adapter types:**

- **Config-based**: Declarative `adapter.yaml` with path templates, command support, and extra frontmatter fields. Uses the built-in generation pipeline with different parameters.
- **Script-based**: External scripts that receive manifest+content as JSON on stdin and return generated files as JSON on stdout. Owns the entire generation pipeline.

**Adapter discovery (precedence order):**

1. User-installed: `~/.skills/adapters/<id>/adapter.yaml` (highest)
2. Skill-bundled: `<package>/adapters/<id>/adapter.yaml`
3. Built-in: compiled into the binary (lowest)

**Generation flow (config-based, v0.1.0):**

```
(Manifest, skill_content: &str, AdapterDef::Config) → Vec<GeneratedFile>
```

1. Read the manifest and resolve content paths
2. Build YAML frontmatter using adapter's path templates and extra fields config
3. Append the skill body unchanged (byte-identical passthrough)
4. If commands are defined and adapter supports them, generate one file per command
5. Return the list of generated files

**Generation flow (config-based, v0.2.0):**

```
(ManifestV2, base_path, AdapterDef::Config) → Vec<GeneratedFile>
```

1. Resolve file globs from `files` list (deduplicated)
2. For each skill in the `skills` map:
   a. Read the skill's entrypoint content
   b. Build frontmatter from skill definition using adapter config
   c. Append skill body
   d. If tools exist, append `## Tools` documentation section
   e. Copy all included files into the skill output directory
   f. Generate wrapper scripts for each tool (`tools/<name>`)
   g. Generate command files (if the skill declares commands and adapter supports them)
3. Mark wrapper scripts executable (chmod +x)

**Generation flow (script-based):**

```
(ManifestAny, content, AdapterDef::Script) → Vec<GeneratedFile>
```

1. Resolve all content (skills, commands, files) into memory
2. Serialize manifest + content + adapter config as JSON
3. Execute the adapter's generate script with JSON on stdin
4. Parse the script's stdout as JSON array of generated files
5. Validate output: no path traversal, relative paths only, size limits
6. Return the generated files

The unified entry point `generate_any()` dispatches to the correct path based on `ManifestAny` and `AdapterDef`.

**Built-in adapters:**

| Adapter | Skill output path | Command output path | Extra fields |
|---------|-------------------|---------------------|--------------|
| `claude-code` | `.claude/skills/{name}/SKILL.md` | `.claude/commands/{namespace}/{command}.md` | — |
| `codex` | `.codex/skills/{name}/SKILL.md` | *(not supported)* | — |
| `pi` | `~/.pi/agent/skills/{name}/SKILL.md` | *(not supported)* | `allowed-tools`, `disable-model-invocation` |

**Key design principle:** The skill body is never transformed. Adapter output = runtime frontmatter + original content. This means:

- Author intent is preserved exactly
- Diffs between runtime outputs show only frontmatter differences
- No risk of content corruption during generation

### aule-resolver

Resolves skill sources to concrete artifacts. Handles version constraints, policy checks, and fetching from multiple sources.

**Resolution sources (in priority order):**

1. **Local path** — `./path/to/skill` — reads directly from filesystem
2. **Cache** — `~/.skills/artifacts/{name}/{version}/` — previously installed
3. **Git URL** — `https://github.com/user/repo` — clones to temp directory, copies to cache
4. **Registry** — `@owner/name` — queries registry API, resolves to git URL

**Key types:**

- `ResolveRequest` — what the user asked for (source, version constraint, target runtime)
- `ResolvePlan` — the resolution result (where the artifact is, which version, what permissions it needs)

**Version resolution:**

The resolver uses `semver` for constraint matching:

```
"^1.0"   → >=1.0.0, <2.0.0
"~1.2"   → >=1.2.0, <1.3.0
"=1.0.0" → exactly 1.0.0
"*"      → any version
```

**Policy enforcement:**

Before installation, the resolver checks the user's policy configuration:

```
ResolveRequest
  → check_allow_list()    skill must match an allow pattern (if configured)
  → check_block_list()    skill must not match any block pattern
  → resolve_source()      fetch from local/cache/git/registry
  → Ok(ResolvePlan)
```

### aule-cache

Manages the local `~/.skills/` directory: artifact storage, metadata indexing, and activation state.

**Cache structure:**

```
~/.skills/
├── config.json           User configuration
├── metadata/
│   └── index.json        Fast lookup index for all installed skills
├── artifacts/
│   └── {name}/
│       └── {version}/
│           ├── skill.yaml
│           ├── content/
│           └── .integrity     SHA-256 hash for verification
└── activation/
    ├── claude-code.json   Active skills for Claude Code
    └── codex.json         Active skills for Codex
```

**Operations:**

| Operation | What it does |
|-----------|--------------|
| `install()` | Copies artifact to cache, updates metadata index, verifies integrity |
| `activate()` | Generates adapter output in project directory, records in activation state |
| `deactivate()` | Removes adapter output, updates activation state |
| `verify()` | Checks SHA-256 hashes against stored `.integrity` files |
| `list_installed()` | Reads metadata index |
| `list_active()` | Reads activation state for a runtime |
| `execute_hook()` | Runs a lifecycle hook script with the package directory as working dir |

**Lifecycle hooks (v0.2.0):**

When installing or activating a v0.2.0 package, the CLI checks for declared hooks and runs them via `execute_hook()`. Hook failure is reported but does not roll back the operation. The `onUninstall` hook runs before file removal.

**Integrity model:**

Each installed artifact stores a `.integrity` file containing the SHA-256 hash of its contents. The `verify()` operation detects:

- Corrupted artifacts (hash mismatch)
- Orphaned artifacts (in filesystem but missing from index)
- Missing artifacts (in index but missing from filesystem)

### aule-infer

Skill inference engine — generates `skill.yaml` for repos that don't have one. Uses a two-stage pipeline.

**Stage 1 (Discovery)** — deterministic, no API key required:

```
scan_all(repo_root)
  ├── ClaudeSkillScanner    .claude/skills/**/*.md   → parse frontmatter
  ├── CodexSkillScanner     .codex/skills/**/*.md    → parse frontmatter
  ├── ClaudeCommandScanner  .claude/commands/**/*.md → extract commands
  ├── PluginScanner         plugin.json              → parse skills/commands
  └── SkillMdScanner        SKILL.md                 → standalone skill files
  → ScanResult { skills, warnings }                  → deduplicate by priority
```

If Stage 1 finds skills, the manifest builder produces a `ManifestV2` mechanically — no LLM call needed.

**Stage 2 (LLM Suggest)** — only runs if Stage 1 finds nothing:

```
gather_signals(repo_root)
  ├── GenericGatherer       README, file tree, license, executables
  ├── NpmGatherer           package.json → name, version, bin entries
  ├── PythonGatherer        pyproject.toml → name, console_scripts
  ├── RustGatherer          Cargo.toml → name, binary targets
  └── GoGatherer            go.mod → module name, cmd/ directory
  → InferredSignals

assess(signals)                           → Claude API call
  → LlmAssessment { can_infer, confidence, suggested_skills, suggested_tools }

build_from_assessment(assessment, signals)
  → ManifestV2                            → validated through aule-schema
```

**Key types:**

- `DiscoveredSkill` — a skill found by a scanner (name, description, entrypoint, commands, source format)
- `InferredSignals` — repo metadata for LLM context (readme, package info, file tree, executables)
- `LlmAssessment` — LLM response with skill/tool suggestions and confidence score
- `InferError` — comprehensive error enum (NoApiKey, LlmUnavailable, LlmRateLimit, LlmResponseParse)

**Design decisions:**

- Stage 1 is always free and fast — no API key, no network calls. This means repos with existing skill artifacts (`.claude/skills/`, `plugin.json`) can be installed instantly.
- The LLM assessor uses Claude Sonnet for cost/speed balance. The system prompt emphasizes that `can_infer: false` is a valid answer — the LLM shouldn't force bad output.
- All output is v0.2.0 `ManifestV2` for consistency, even single-skill results.
- The builder validates output by round-tripping through `aule-schema`'s parser.

### aule-cli

The user-facing binary. Maps subcommands to library calls.

**Structure:**

```
src/
├── main.rs          CLI definition (clap derive)
├── commands/
│   ├── init.rs      Scaffold new skill package
│   ├── validate.rs  Validate manifest and contract (v0.1.0 + v0.2.0)
│   ├── build.rs     Generate adapter output (v0.1.0 + v0.2.0)
│   ├── migrate.rs   Convert v0.1.0 manifest to v0.2.0
│   ├── infer.rs     Infer skill.yaml from repo content
│   ├── install.rs   Install from any source (runs onInstall hook, supports --infer)
│   ├── activate.rs  Bind skill to runtime (runs onActivate hook)
│   ├── list.rs      List installed/active skills
│   ├── login.rs     GitHub OAuth flow
│   ├── logout.rs    Remove auth token
│   ├── publish.rs   Register with registry
│   └── search.rs    Query registry
├── output.rs        Formatting (human-readable vs JSON)
└── registry.rs      Registry API client
```

**Output modes:**

Every command supports `--json` for machine-readable output. The `output.rs` module handles the switch:

```rust
if json_output {
    println!("{}", serde_json::to_string_pretty(&result)?);
} else {
    // Human-readable formatting
}
```

Errors also respect `--json`, outputting `{ "error": "...", "code": N }` and exiting with the appropriate code.

## Data Flow

### Full lifecycle: author → install → activate

```
1. AUTHOR                     2. VALIDATE                   3. BUILD
   skill.yaml                    skill validate                skill build
   content/skill.md              → aule-schema                 → aule-adapter
                                 → Ok / Err                    → .claude/skills/SKILL.md

4. PUBLISH                    5. INSTALL                    6. ACTIVATE
   skill publish                 skill install @author/name    skill activate name
   → registry API                → aule-resolver               → aule-cache
   → registers git URL           → aule-cache (store)          → generates adapter files
                                 → policy check                → updates activation state
```

### Adapter generation detail

```
Input:                              Output (.claude/skills/my-skill/SKILL.md):
┌──────────────────────┐            ┌──────────────────────┐
│ skill.yaml           │            │ ---                  │
│  name: my-skill      │───────────▶│ name: my-skill       │  ← mapped from manifest
│  description: ...    │            │ description: ...     │
│  metadata:           │            │ license: MIT         │
│    author: me        │            │ metadata:            │
│    license: MIT      │            │   author: me         │
│                      │            │ ---                  │
│ content/skill.md     │            │                      │
│  Do the thing...     │───────────▶│ Do the thing...      │  ← byte-identical passthrough
└──────────────────────┘            └──────────────────────┘
```

## Adding a New Runtime Adapter

The adapter system is pluggable — you can add support for a new AI coding agent without modifying the Aulë source code.

### Config-based adapter (most runtimes)

Create an `adapter.yaml` file:

```yaml
id: my-runtime
type: config
protocol: 1
description: "Adapter for My Runtime"
paths:
  skill: ".my-runtime/skills/{name}/SKILL.md"
  commands:                              # optional
    path: ".my-runtime/commands/{namespace}/{command_name}.md"
    display_name: "{skill}: {command}"
    category: "Workflow"
    tags: ["workflow", "{skill}", "{command}"]
frontmatter:
  extra_fields: []                       # fields from AdapterConfig.extra to include
```

Install it: `skill adapters add --path ./my-adapter/`

### Script-based adapter (custom output formats)

For runtimes that don't use SKILL.md:

```yaml
id: my-runtime
type: script
protocol: 1
description: "Adapter for My Runtime"
generate: ./generate.py
validate: ./validate.py    # optional
```

The generate script receives JSON on stdin (manifest + content) and writes JSON to stdout (array of files).

### CLI commands

```bash
skill adapters list              # See all available adapters
skill adapters add --path ./dir  # Install from local directory
skill adapters add --git <url>   # Install from git repo
skill adapters remove <id>       # Remove user-installed adapter
skill adapters info <id>         # Show adapter details
skill adapters test <id>         # Test adapter correctness
```

## Testing Strategy

```
aule-schema     48 tests   Unit: parsing (v0.1.0 + v0.2.0), validation, edge cases
aule-adapter    55 tests   Unit + integration: adapter definitions, registry, script protocol,
                            v0.1.0/v0.2.0 generation, wrapper scripts, tool docs, real skill validation
aule-resolver   18 tests   Unit: resolution from each source, policy enforcement, v0.2.0 manifests
aule-cache      20 tests   Unit: install, activate, integrity, hooks, index operations
aule-infer      38 tests   Unit: scanners, gatherers, assessor, builder, round-trip validation
aule-cli        23 tests   Integration: end-to-end CLI tests including infer and install --infer
                ──────
                ~204 total
```

The critical test is `real_skills_test.rs` in `aule-adapter`. It generates adapter output for all example skills (six v0.1.0, one v0.2.0) and asserts byte-for-byte equality with the committed output in `.claude/` and `.codex/`. The v0.2.0 test also verifies wrapper scripts, `## Tools` sections, and included file copying.
