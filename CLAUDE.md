# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Aulë** is an open, runtime-agnostic skill ecosystem — a protocol and toolchain for discoverable, versioned, composable agent/coding skills. Think Go Modules + npm + OpenAPI + VS Code Extensions for AI agent capabilities.

## Build & Test Commands

```bash
cargo build                     # Build all crates
cargo test                      # Run all 96 tests
cargo test -p aule-schema       # Test one crate
cargo test -p aule-adapter --test real_skills_test  # Run real skill validation
cargo run -p aule-cli -- --help # Run the CLI
```

The binary is named `skill` (defined in `aule-cli/Cargo.toml`).

## Repository Structure

```
crates/
  aule-schema/     — Protocol types: manifest, contract, permissions, envelope, metadata
  aule-adapter/    — Adapter generator: manifest → runtime-specific SKILL.md files
  aule-resolver/   — Skill resolution: local path, cache, git URL, semver constraints
  aule-cache/      — Local ~/.skills/ cache: artifacts, metadata index, activation state
  aule-cli/        — `skill` binary: init, validate, build, install, activate, list
examples/          — Example skill packages demonstrating the skill format and CLI usage
openspec/          — OpenSpec change management artifacts (proposal, design, specs, tasks)
.claude/, .codex/  — Generated adapter output for Claude Code and Codex runtimes
```

## Architecture

**Cargo workspace** with 5 crates. Library crates do the work; the CLI binary is a thin wrapper.

```
aule-cli (binary)
  ├── aule-schema    (manifest parsing, contract validation, permissions)
  ├── aule-adapter   (generates SKILL.md per runtime target)
  ├── aule-resolver  (version resolution, policy checks, git clone)
  └── aule-cache     (artifact storage, activation state)
```

**Skill source format:** A skill is a directory with `skill.yaml` (manifest) + `content/skill.md` (body). The adapter generator reads the manifest and produces runtime-specific files in `.claude/skills/`, `.codex/skills/`, etc.

**Key design choice:** Adapter output is template + transform, not codegen. The skill body passes through byte-identical; only frontmatter is mapped per-runtime.

**Validation gate:** `crates/aule-adapter/tests/real_skills_test.rs` generates all example skills and asserts output matches the existing hand-written adapter files.

## OpenSpec Workflow

```bash
openspec list --json                                    # List active changes
openspec status --change "<name>" --json                # Check change status
openspec new change "<name>"                            # Create a new change
openspec instructions <artifact-id> --change "<name>" --json  # Get artifact instructions
```

Changes live in `openspec/changes/<name>/` with: `proposal.md`, `design.md`, `specs/`, `tasks.md`.

## Key Domain Concepts

- **Manifest** (`skill.yaml`) — Single source of truth for a skill's identity, contract, adapters, and dependencies
- **Contract** — Versioned interface: inputs/outputs (prompt or JSON Schema), permissions, determinism bounds
- **RuntimeTarget** — Defines directory layout and frontmatter mapping for a coding agent (Claude Code, Codex)
- **Resolver** — Selects version + adapter + artifact from local path, cache, or git URL
- **Activation** — Binding an installed skill to a specific runtime by generating adapter files
