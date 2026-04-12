# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Aule** is a design/planning repository for a **Skill Registry Ecosystem** — an open, runtime-agnostic protocol and first-party platform for discoverable, versioned, composable agent/coding skills. Think Go Modules + npm + OpenAPI + VS Code Extensions for AI agent capabilities.

This repo contains no application code yet. It holds architectural documents, specification summaries, and OpenSpec-driven workflow artifacts used to plan and design the system.

## Repository Structure

- `notes/` — Architectural reference documents:
  - `skill_registry_ecosystem_architecture.md` — Blueprint defining the open protocol layer (domain model, identity/resolution, versioning, contracts, packaging, trust/security)
  - `skill_registry_product_architecture_doc.md` — Product architecture mapping the blueprint into services, APIs, storage, and runtime components
  - `skill_registry_spec_generation_summary.md` — Condensed brief used as input for generating full technical specifications
- `openspec/` — OpenSpec workflow directory:
  - `config.yaml` — OpenSpec configuration (currently using `spec-driven` schema)
  - `changes/` — Active and archived change proposals
  - `specs/` — Accumulated specifications (currently empty)
- `.claude/`, `.codex/`, `.opencode/`, `.pi/` — OpenSpec skills and commands for multiple AI coding agents

## OpenSpec Workflow

This project uses **OpenSpec** for structured change management. The workflow is:

1. **Explore** (`/opsx:explore`) — Think through ideas without implementing
2. **Propose** (`/opsx:propose`) — Create a change with proposal, design, and tasks artifacts
3. **Apply** (`/opsx:apply`) — Implement tasks from a change
4. **Archive** (`/opsx:archive`) — Archive completed changes

Changes live in `openspec/changes/<name>/` and contain `proposal.md`, `design.md`, `tasks.md`, and optionally delta specs under `specs/`.

Key commands:
```bash
openspec list --json          # List active changes
openspec status --change "<name>" --json  # Check change status
openspec new change "<name>"  # Create a new change
openspec instructions <artifact-id> --change "<name>" --json  # Get artifact instructions
```

## Key Architectural Concepts

The skill ecosystem design is built around these core domain objects — understanding them is essential when working with the spec documents:

- **Skill** — Discoverable logical capability with identity, taxonomy, and metadata
- **Contract** — Versioned deterministic interface (input/output schemas, permissions, guarantees)
- **Implementation** — Executable realization of a contract (language/runtime-agnostic)
- **Adapter** — Runtime-specific plugin binding a host runtime to an implementation
- **Artifact** — Distributable form (source, binary, wasm, container, bundle)
- **Resolver** — Policy-driven engine selecting version + adapter + implementation + artifact

The system has four planes: Publisher, Platform, Runtime, and Local Machine.
