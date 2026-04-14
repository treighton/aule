# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Aulë** is an open, runtime-agnostic skill ecosystem — a protocol and toolchain for discoverable, versioned, composable agent/coding skills. Think Go Modules + npm + OpenAPI + VS Code Extensions for AI agent capabilities.

## Build & Test Commands

```bash
cargo build                     # Build all crates
cargo test                      # Run all ~154 tests
cargo test -p aule-schema       # Test one crate
cargo test -p aule-adapter --test real_skills_test  # Run real skill validation
cargo run -p aule-cli -- --help # Run the CLI
```

The binary is named `skill` (defined in `aule-cli/Cargo.toml`).

## Repository Structure

```
crates/
  aule-schema/     — Protocol types: manifest (v0.1.0 + v0.2.0), contract, permissions, envelope, metadata
  aule-adapter/    — Pluggable adapter system: manifest → SKILL.md via config-based or script-based adapters
  aule-resolver/   — Skill resolution: local path, cache, git URL, semver constraints
  aule-cache/      — Local ~/.skills/ cache: artifacts, metadata index, activation state, hook execution
  aule-cli/        — `skill` binary: init, validate, build, migrate, install, activate, list, adapters
examples/          — Example skill packages demonstrating the skill format and CLI usage
openspec/          — OpenSpec change management artifacts (proposal, design, specs, tasks)
.claude/, .codex/  — Generated adapter output for Claude Code and Codex runtimes
```

## Architecture

**Cargo workspace** with 5 crates. Library crates do the work; the CLI binary is a thin wrapper.

```
aule-cli (binary)
  ├── aule-schema    (manifest parsing, contract validation, permissions)
  ├── aule-adapter   (pluggable adapter system, generates SKILL.md per adapter)
  ├── aule-resolver  (version resolution, policy checks, git clone)
  └── aule-cache     (artifact storage, activation state)
```

**Skill source format:**
- **v0.1.0**: `skill.yaml` + `content/skill.md` — single skill, prose only
- **v0.2.0**: `skill.yaml` + `content/` + `logic/` — multi-skill packages with executable tools, typed I/O, lifecycle hooks

**Key design choice:** Adapter output is template + transform, not codegen. The skill body passes through byte-identical; only frontmatter is mapped per-runtime. For v0.2.0, the adapter also generates wrapper scripts, tool documentation, and copies bundled files.

**Validation gate:** `crates/aule-adapter/tests/real_skills_test.rs` generates all example skills (v0.1.0 + v0.2.0) and asserts output matches the committed adapter files.

## OpenSpec Workflow

```bash
openspec list --json                                    # List active changes
openspec status --change "<name>" --json                # Check change status
openspec new change "<name>"                            # Create a new change
openspec instructions <artifact-id> --change "<name>" --json  # Get artifact instructions
```

Changes live in `openspec/changes/<name>/` with: `proposal.md`, `design.md`, `specs/`, `tasks.md`.

## Key Domain Concepts

- **Manifest** (`skill.yaml`) — Single source of truth for a skill's identity, interface, adapters, and dependencies. Two versions:
  - **v0.1.0** (`Manifest`): `content` + `contract` — single-skill, prose-only
  - **v0.2.0** (`ManifestV2`): `files` + `skills` + `tools` + `hooks` — multi-skill packages with executable tools
- **ManifestAny** — Version-dispatched enum used by parser and adapter: `V1(Manifest)` | `V2(ManifestV2)`
- **SkillDefinition** — Per-skill interface in v0.2.0 (entrypoint, description, version, inputs, outputs, permissions, determinism)
- **Tool** — Executable tool declaration (runtime, entrypoint, typed JSON Schema input/output)
- **Hooks** — Lifecycle scripts (onInstall, onActivate, onUninstall) executed by the CLI
- **Contract** — Versioned interface (v0.1.0 only): inputs/outputs (prompt or JSON Schema), permissions, determinism bounds
- **AdapterDef** — Defines how a skill is transformed for a runtime. Two types:
  - **Config-based** (`AdapterDef::Config`): Declarative path templates and frontmatter config, uses the built-in generation pipeline
  - **Script-based** (`AdapterDef::Script`): External scripts that own the entire pipeline via stdin/stdout JSON protocol
- **AdapterRegistry** — Discovers adapters from three sources with precedence: user-installed (`~/.skills/adapters/`) > skill-bundled (`<package>/adapters/`) > built-in (compiled). Built-in adapters (claude-code, codex, pi) are expressed as config-based definitions.
- **adapter.yaml** — Adapter definition file declaring id, type (config/script), path templates, extra frontmatter fields, optional validate/generate scripts
- **Resolver** — Selects version + adapter + artifact from local path, cache, or git URL
- **Activation** — Binding an installed skill to a specific runtime by generating adapter files
