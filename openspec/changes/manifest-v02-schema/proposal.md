## Why

The v0.1.0 manifest schema models skills as single-file prose instructions — one `skill.yaml` maps to one `content/skill.md`. This works for simple prompt-based skills but can't express skills that ship executable code (test generators, data processors, CLI tools), need lifecycle setup (npm install, runtime verification), or want to bundle multiple related skills in a single distributable package. As the ecosystem grows toward more sophisticated skills with deterministic tool use and agentic loops, the manifest needs to support executable logic, typed tool definitions, lifecycle hooks, and multi-skill packages.

## What Changes

- **BREAKING**: Rename `contract` to `skill` (v0.1.0) → `skills` (v0.2.0) — a map of named skill definitions, each with its own entrypoint, interface (inputs/outputs/permissions/determinism/errors), enabling multiple skills per package
- **BREAKING**: Replace `content` with `files` — a flat list of include globs declaring all files bundled with the package (skill markdown, tool scripts, templates, configs)
- Add top-level `tools` — a map of named executable tools that skills can invoke, each declaring its own runtime (`using`), version constraint, entrypoint, and typed input/output schemas (JSON Schema). The adapter generates wrapper scripts and documentation so agents call tools without knowing the underlying runtime.
- Add top-level `hooks` — lifecycle event handlers (`onInstall`, `onActivate`, `onUninstall`) that the system executes at defined moments, solving setup concerns like dependency installation and runtime verification
- Bump `schemaVersion` from `"0.1.0"` to `"0.2.0"` — the parser uses this to select the correct schema shape; v0.1.0 manifests continue to work unmodified
- Adapter generates wrapper scripts (shell shims) for each tool at build time, plus a `## Tools` documentation section appended to each generated SKILL.md
- Skill entrypoints move from `content.skill` (implicit single path) to `skills.<name>.entrypoint` (explicit per-skill path), enabling multiple skills with distinct interfaces in one package

## Capabilities

### New Capabilities
- `executable-tools`: Top-level `tools` map in the manifest — named executable tools with per-tool runtime declaration (`using`, `version`), entrypoint path, typed JSON Schema input/output, and adapter-generated wrapper scripts for runtime-agnostic invocation
- `lifecycle-hooks`: Top-level `hooks` map — `onInstall`, `onActivate`, `onUninstall` event handlers that the system executes during skill lifecycle transitions (e.g., running `npm install` after installation)
- `multi-skill-packages`: Evolve the manifest from 1:1 (one manifest = one skill) to 1:N (one manifest = one package = N skills), where each skill has its own entrypoint, interface, permissions, and optional commands
- `wrapper-script-generation`: Adapter generates shell wrapper scripts for each declared tool at build time, abstracting the runtime so agents invoke tools via simple executables without knowing the underlying language
- `example-api-contract-tester`: Reference implementation skill package demonstrating all new capabilities — executable Node.js tools (generate, run-tests, report), lifecycle hooks, multi-phase agentic workflow with deterministic code + agentic diagnosis loop

### Modified Capabilities
- `manifest-schema`: **BREAKING** — `contract` renamed/restructured to `skills` (map of named skills with entrypoints), `content` replaced by `files` (include glob list), new top-level `tools` and `hooks` fields, `schemaVersion` bumped to `"0.2.0"`
- `adapter-generator`: Extended to copy `files` includes, generate wrapper scripts for tools, append tool documentation to SKILL.md, and run lifecycle hooks during install/activate

## Impact

- **Schema crate** (`aule-schema`): Major changes to manifest types — new `Skills`, `Tool`, `Hook` structs, updated validation, new schema version handling, backward-compatible parsing for v0.1.0
- **Adapter crate** (`aule-adapter`): New wrapper script generation, tool documentation generation, file copying from include globs, updated frontmatter mapping
- **Cache crate** (`aule-cache`): Hook execution during install/activate lifecycle, storing hook state
- **CLI** (`aule-cli`): `skill build` generates wrappers and copies includes, `skill install` triggers `onInstall` hook, `skill activate` triggers `onActivate` hook
- **Existing examples**: Must be updated to v0.2.0 schema shape (or kept as v0.1.0 with backward-compat parsing)
- **Registry platform** (`platform/`): TypeScript manifest parser must support v0.2.0 schema alongside v0.1.0
- **JSON Schema** (`manifest.schema.json`): Must be versioned or support both shapes via conditional schemas
