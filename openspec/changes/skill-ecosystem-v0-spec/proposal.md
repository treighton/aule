## Why

There is no open, runtime-agnostic standard for packaging, discovering, installing, and invoking skills across coding agents. Today, skills are manually duplicated across runtime-specific directories (`.claude/`, `.codex/`, `.opencode/`, `.pi/`) with identical content and slightly different structural conventions. This project defines the core protocol schemas and local toolchain needed to publish a skill once and generate correct adapters for multiple runtimes automatically. Starting now because the skill/plugin ecosystem across coding agents is fragmenting rapidly — establishing a protocol early means shaping the standard rather than adapting to competing proprietary ones.

## What Changes

- Define a **manifest schema** (YAML/JSON) that describes a skill's identity, capabilities, contracts, adapters, dependencies, and permissions
- Define a **contract schema** for versioned deterministic interfaces (input/output schemas, permissions, guarantees)
- Define a **metadata endpoint protocol** (`.well-known/skill.json`) for decentralized identity resolution
- Define an **invocation envelope** spec for request/response/error boundaries between adapters and implementations
- Define a **permission vocabulary** v0.1 for declarative capability/permission declarations
- Design an **adapter generator** that produces runtime-specific skill packages for Claude Code and Codex from a single skill source
- Design a **CLI tool** (`skill init`, `build`, `validate`, `install`, `activate`) as the primary developer interface
- Design a **local cache and activation manager** for machine-level artifact storage and per-runtime enablement
- Design a **resolver** for version/adapter/artifact selection with policy inputs

## Capabilities

### New Capabilities
- `manifest-schema`: YAML/JSON schema defining skill identity, metadata, contract references, adapter sets, implementations, artifacts, dependencies, permissions, and extension namespaces
- `contract-schema`: Versioned interface specification — input/output schemas, required permissions/capabilities, determinism bounds, error model, optional behavioral and workflow metadata
- `metadata-endpoint`: `.well-known/skill.json` protocol for decentralized skill identity resolution — endpoint format, resolution algorithm, required/optional fields, caching semantics
- `invocation-envelope`: Canonical request/response/error envelope format for the adapter-implementation boundary
- `permission-vocabulary`: Granular permission and host capability vocabulary (filesystem, network, process, etc.) with trust metadata model
- `adapter-generator`: Rules and templates for generating runtime-specific skill packages — Claude Code and Codex target formats, directory layout, frontmatter mapping, content transformation
- `cli-tool`: Developer-facing CLI for skill authoring lifecycle — init, build, validate, install, activate, publish commands
- `resolver`: Resolution algorithm for selecting skill version, contract, adapter, implementation, and artifact given environment and policy constraints
- `cache-manager`: Local machine storage layout for installed artifacts, metadata cache, activation state per runtime, and install/activation separation

### Modified Capabilities

(None — greenfield project, no existing specs)

## Impact

- New Rust Cargo workspace to be created with crates for: schema parser/validator (`aule-schema`), resolver library (`aule-resolver`), adapter generator (`aule-adapter`), cache manager (`aule-cache`), CLI binary (`aule-cli`)
- Distributes as a single static binary (`skill`) with no runtime dependencies — runtime plugins in any language integrate via CLI subprocess + JSON output
- Protocol schemas (manifest, contract, metadata endpoint, invocation envelope) become the foundation that all other components depend on — changes to these are high-cost after v0.1
- Adapter generator directly targets Claude Code (`.claude/skills/`, `.claude/commands/`) and Codex (`.codex/skills/`) directory structures and frontmatter formats
- CLI tool becomes the primary developer interface for the entire skill authoring and consumption workflow
- OpenSpec skills in this repo (`openspec-explore`, `openspec-propose`, etc.) serve as validation targets — the adapter generator should reproduce their current multi-runtime structure from a single source
