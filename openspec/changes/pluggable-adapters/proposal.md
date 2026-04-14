## Why

The adapter system is hard-wired to three built-in runtimes (Claude Code, Codex, Pi) via a match statement in `RuntimeTarget::by_id()`. Adding support for a new runtime — Gemini, Cursor, Windsurf, or any community-built coding agent — requires modifying the Rust source, recompiling, and releasing a new version. This bottleneck prevents community-driven runtime support, which is essential for an ecosystem that aims to be runtime-agnostic. The skill manifest already accepts arbitrary adapter IDs via `HashMap<String, AdapterConfig>`, but the generation pipeline silently ignores any ID it doesn't recognize.

## What Changes

- Add an `adapter.yaml` schema for defining external adapters, supporting two types:
  - **Config-based** (declarative): path templates, command support, extra frontmatter fields — uses the existing generation pipeline with different parameters
  - **Script-based** (full control): external scripts that receive manifest+content as JSON on stdin and return generated files as JSON on stdout — owns the entire generation pipeline
- Add an adapter registry that discovers adapters from three sources with precedence: user-installed (`~/.skills/adapters/`), skill-bundled (`<package>/adapters/`), built-in (compiled)
- Refactor built-in adapters (claude-code, codex, pi) to be expressed as config-based adapter definitions, eliminating special-casing like `PI_EXTRA_FIELDS`
- Add a versioned JSON protocol for script-based adapter communication (stdin/stdout), with a `protocol_version` field for forward compatibility
- Support optional validation scripts that adapter authors can provide to check manifest compatibility before generation
- Add CLI commands: `skill adapters list`, `skill adapters add`, `skill adapters remove`, `skill adapters info`, `skill adapters test`
- `skill build --target <id>` resolves custom adapter IDs through the registry instead of failing on unknown targets

## Capabilities

### New Capabilities
- `adapter-definition-schema`: The `adapter.yaml` schema for declaring external adapters — identity (id, description, author, protocol version), type (config or script), path templates, command support, extra frontmatter fields, and optional validate/generate script references
- `adapter-registry`: Discovery and precedence-ordered resolution of adapters from user-installed, skill-bundled, and built-in sources — replaces the hard-coded `by_id()` match and `all_known()` vec
- `script-adapter-protocol`: Versioned JSON protocol for script-based adapters — input schema (manifest, content, adapter config, options), output schema (generated files), error reporting, and protocol version negotiation
- `adapter-validation`: Optional validation scripts that adapter authors provide to check skill manifest compatibility before generation — structured error/warning output for pre-flight checks
- `adapter-cli-commands`: CLI surface for managing adapters — `list`, `add` (from path or git), `remove`, `info`, and `test` commands under `skill adapters`

### Modified Capabilities
- `adapter-generator`: Refactored to dispatch between config-based (built-in pipeline with external parameters) and script-based (subprocess call) adapters. Built-in adapters become config-based definitions. `PI_EXTRA_FIELDS` eliminated in favor of per-adapter `extra_fields` config.

## Impact

- **Adapter crate** (`aule-adapter`): Major refactor — `RuntimeTarget` replaced by `AdapterDef` enum (Config | Script), new `AdapterRegistry` for discovery/resolution, generation pipeline parameterized by adapter config, script execution via subprocess
- **Schema crate** (`aule-schema`): New `adapter.yaml` parsing types (or in adapter crate, depending on layering)
- **CLI** (`aule-cli`): New `skill adapters` subcommand group (list, add, remove, info, test), `skill build --target` updated to use registry lookup
- **Cache crate** (`aule-cache`): May need to store installed adapter state in `~/.skills/adapters/`
- **Existing adapters**: claude-code, codex, pi continue to work unchanged from user perspective, but are internally expressed as config-based definitions
- **Existing skills**: No changes required — `adapters:` section in manifests already accepts arbitrary string keys
- **Third-party ecosystem**: Community can publish adapter packages (directories with `adapter.yaml` + optional scripts) for new runtimes
