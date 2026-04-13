# Architecture

This document describes Aulë's internal architecture for contributors and anyone integrating with the protocol.

## Overview

Aulë is a Cargo workspace with five crates organized as a library-first design. The CLI binary (`skill`) is a thin wrapper — all logic lives in the library crates.

```
aule-cli (binary)
  ├── aule-schema      Protocol types, parsing, validation
  ├── aule-adapter     Runtime adapter generation
  ├── aule-resolver    Multi-source version resolution
  └── aule-cache       Local artifact storage and activation
```

## Crate Details

### aule-schema

The foundation. Defines all protocol types and validation logic.

**Key types:**

- `Manifest` — parsed representation of `skill.yaml`
- `Contract` — versioned interface declaration (inputs, outputs, permissions, determinism)
- `Permission` — capability requirement with scope (e.g., `filesystem.read`, `process.spawn`)
- `RequestEnvelope` / `ResponseEnvelope` — invocation protocol for structured skill calls
- `Metadata` — author, license, version, generation info

**Validation pipeline:**

```
Raw YAML string
  → serde_yaml::from_str::<Manifest>()
  → validate_schema_version()        must be "0.1.0"
  → validate_name()                  alphanumeric + hyphens
  → validate_version()               semantic version
  → validate_contract()
      → validate_permissions()       against known vocabulary
      → validate_inputs/outputs()    "prompt" or valid JSON Schema
      → validate_determinism()       deterministic | bounded | probabilistic
  → validate_adapters()              known runtime targets
  → Ok(Manifest)
```

**Design decisions:**

- Permissions use a flat `scope.action` vocabulary rather than hierarchical namespaces. This keeps policy matching simple and avoids ambiguity.
- The envelope types (`RequestEnvelope`, `ResponseEnvelope`) support both prompt-based and JSON Schema-based contracts. Most skills use `"prompt"` — structured contracts are for machine-to-machine skill composition.
- Schema version is pinned to `"0.1.0"` during the protocol stabilization phase.

### aule-adapter

Generates runtime-specific output files from a manifest and skill content.

**Key types:**

- `RuntimeTarget` — defines the output layout for a specific agent (directory structure, frontmatter mapping)
- `GeneratedFile` — a path + content pair produced by the generator

**Generation flow:**

```
(Manifest, skill_content: &str, RuntimeTarget) → Vec<GeneratedFile>
```

1. Read the manifest and resolve content paths
2. Build YAML frontmatter for the target runtime (name, description, license, metadata, compatibility)
3. Append the skill body unchanged (byte-identical passthrough)
4. If commands are defined, generate one file per command in the target's command directory
5. Return the list of generated files

**Runtime targets currently defined:**

| Target | Skill output path | Command output path |
|--------|-------------------|---------------------|
| `claude-code` | `.claude/skills/{name}/SKILL.md` | `.claude/commands/{namespace}/{command}.md` |
| `codex` | `.codex/skills/{name}/SKILL.md` | *(not supported)* |

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

**Integrity model:**

Each installed artifact stores a `.integrity` file containing the SHA-256 hash of its contents. The `verify()` operation detects:

- Corrupted artifacts (hash mismatch)
- Orphaned artifacts (in filesystem but missing from index)
- Missing artifacts (in index but missing from filesystem)

### aule-cli

The user-facing binary. Maps subcommands to library calls.

**Structure:**

```
src/
├── main.rs          CLI definition (clap derive)
├── commands/
│   ├── init.rs      Scaffold new skill package
│   ├── validate.rs  Validate manifest and contract
│   ├── build.rs     Generate adapter output
│   ├── install.rs   Install from any source
│   ├── activate.rs  Bind skill to runtime
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

## Adding a New Runtime Target

To support a new AI coding agent:

1. **Define the target** in `aule-adapter/src/target.rs`:
   - Output directory layout (where SKILL.md goes)
   - Frontmatter field mapping (what the agent expects)
   - Command support (if applicable)

2. **Add the target name** to `aule-schema`'s known adapters list

3. **Add activation support** in `aule-cache` for the new runtime

4. **Write tests** — add cases to `real_skills_test.rs` for the new target

5. **Generate reference output** — build the included skills for the new target and commit the output

The adapter system is designed to make this straightforward. Most of the work is defining the frontmatter mapping — the content passthrough is automatic.

## Testing Strategy

```
aule-schema     36 tests   Unit: parsing, validation, edge cases
aule-adapter     8 tests   Unit + integration: generation, real skill validation
aule-resolver   18 tests   Unit: resolution from each source, policy enforcement
aule-cache      17 tests   Unit: install, activate, integrity, index operations
aule-cli        14 tests   Integration: end-to-end CLI tests with temp directories
                ──────
                ~97 total
```

The critical test is `real_skills_test.rs` in `aule-adapter`. It generates adapter output for all four included skills and asserts byte-for-byte equality with the committed output in `.claude/` and `.codex/`. This catches any unintended changes to the generation pipeline.
