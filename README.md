# Aulë

**An open, runtime-agnostic skill ecosystem for AI coding agents.**

Named after [Aulë the Smith](https://tolkiengateway.net/wiki/Aul%C3%AB) — the Vala of craftsmanship in Tolkien's legendarium, master of all crafts and forger of the substance of the world. As Aulë shaped the matter of Arda, this project shapes the building blocks for AI agents.

Aulë is a protocol and toolchain for discoverable, versioned, composable skills — reusable capabilities that any AI coding agent can consume. Think Go Modules meets npm, but for agent skills.

Write a skill once. Publish it. Any compatible agent — Claude Code, Codex, or others — can install and use it without modification.

---

## Why Aulë?

AI coding agents are powerful, but their skills are siloed. A prompt workflow that works in Claude Code doesn't transfer to Codex. There's no way to version, share, or discover what others have built.

Aulë fixes this with:

- **A standard manifest format** (`skill.yaml`) — declare what your skill does, what it needs, and how it behaves
- **Runtime adapters** — one skill source generates output for Claude Code, Codex, and future agents
- **Semantic versioning** — depend on skills the way you depend on packages
- **A permission model** — skills declare capabilities they need; users control what's allowed
- **A registry** — discover, install, and publish skills from the command line

## Quick Start

### Install

```bash
# Clone and build from source
git clone https://github.com/treightonmauldin/aule.git
cd aule
cargo build --release

# The binary is called `skill`
./target/release/skill --help
```

### Create a skill

```bash
skill init --name my-skill
```

This scaffolds a new skill package:

```
my-skill/
  skill.yaml          # Manifest — identity, contract, adapters
  content/
    skill.md           # Skill body — what the agent sees
```

### Validate and build

```bash
# Check the manifest and contract are well-formed
skill validate --path my-skill/

# Generate adapter output for Claude Code
skill build --path my-skill/ --target claude-code --output .claude/skills/
```

### Install a skill

```bash
# From a local directory
skill install ./path/to/skill

# From a git repository
skill install https://github.com/user/skill-repo --ref main

# From the registry
skill install @author/skill-name --version "^1.0"
```

### Activate for a runtime

```bash
# Bind an installed skill to Claude Code
skill activate my-skill --target claude-code

# See what's active
skill list --active
```

## How It Works

```
              skill.yaml + content/ + logic/
                               │
                               ▼
                    ┌─────────────────────┐
                    │    aule-schema      │  Parse & validate manifest
                    │                     │  (v0.1.0 or v0.2.0)
                    └─────────┬───────────┘
                              │
                              ▼
                    ┌─────────────────────┐
                    │    aule-adapter     │  Generate SKILL.md per skill,
                    │                     │  wrapper scripts, tool docs,
                    │                     │  copy bundled files
                    └─────────┬───────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
     .claude/skills/   .codex/skills/    (future runtimes)
      SKILL.md          SKILL.md
      tools/            tools/
      logic/            logic/
```

The key design principle: **your skill body passes through unchanged**. Only the YAML frontmatter is adapted per runtime. For v0.2.0, the adapter also generates wrapper scripts, appends tool documentation, and copies included files.

## Skill Manifest

A `skill.yaml` declares everything about a skill. Aulë supports two schema versions:

### v0.1.0 — Single-skill packages (prose-only)

```yaml
schemaVersion: "0.1.0"
name: "my-skill"
description: "A brief description for discovery and triggering"
version: "1.0.0"

content:
  skill: "content/skill.md"

contract:
  version: "1.0.0"
  inputs: "prompt"
  outputs: "prompt"
  permissions:
    - "filesystem.read"
  determinism: "probabilistic"

adapters:
  claude-code:
    enabled: true
  codex:
    enabled: true

metadata:
  author: "your-name"
  license: "MIT"
```

### v0.2.0 — Multi-skill packages with executable tools

v0.2.0 adds support for multiple skills per package, executable tools with typed I/O, and lifecycle hooks:

```yaml
schemaVersion: "0.2.0"
name: "api-testing-suite"
description: "API contract testing with executable tools"
version: "1.0.0"

files:                          # Glob patterns for bundled files
  - "content/**"
  - "logic/**"

skills:                         # Map of skill name → definition
  contract-tester:
    description: "Generate and run contract tests"
    entrypoint: "content/contract-tester.md"
    version: "1.0.0"
    permissions: ["filesystem.read", "process.spawn", "network.external"]
    determinism: "bounded"

  spec-linter:
    description: "Validate an OpenAPI spec"
    entrypoint: "content/spec-linter.md"
    version: "1.0.0"
    permissions: ["filesystem.read"]
    determinism: "deterministic"

tools:                          # Executable tools with typed I/O
  generate:
    description: "Generate test harness from an OpenAPI spec"
    using: "node"
    version: ">= 18"
    entrypoint: "logic/tools/generate.ts"
    input:
      type: "object"
      properties:
        spec: { type: "string" }
      required: ["spec"]
    output:
      type: "object"
      properties:
        status: { type: "string" }
        testCount: { type: "integer" }

hooks:                          # Lifecycle scripts
  onInstall: "logic/hooks/setup.sh"
  onActivate: "logic/hooks/verify-runtime.sh"

adapters:
  claude-code:
    enabled: true
  codex:
    enabled: true
```

The adapter generates wrapper scripts for tools, appends tool documentation to SKILL.md, and copies included files into the output directory.

To migrate an existing v0.1.0 manifest: `skill migrate --path my-skill/`

### Permission Vocabulary

Skills must declare the capabilities they need. Users can enforce policies that block skills requiring permissions they haven't approved.

| Permission | Scope |
|------------|-------|
| `filesystem.read` | Read files from disk |
| `filesystem.write` | Write or modify files |
| `process.spawn` | Execute shell commands |
| `network.external` | Make outbound network requests |

## Architecture

Aulë is a Cargo workspace with five crates. Library crates do the work; the CLI is a thin wrapper.

```
aule-cli (binary: `skill`)
  ├── aule-schema      Manifest parsing, contract validation, permissions
  ├── aule-adapter     Generates runtime-specific SKILL.md files
  ├── aule-resolver    Version resolution, policy checks, git clone
  └── aule-cache       Artifact storage (~/.skills/), activation state
```

### Crates

| Crate | Role | Key Types |
|-------|------|-----------|
| **aule-schema** | Protocol types and validation | `Manifest`, `ManifestV2`, `ManifestAny`, `Contract`, `SkillDefinition`, `Tool`, `Hooks` |
| **aule-adapter** | Pluggable adapter system | `AdapterDef`, `AdapterRegistry`, `GeneratedFile`, `generate()`, `generate_v2()`, `generate_any()` |
| **aule-resolver** | Skill resolution from multiple sources | `ResolveRequest`, `ResolvePlan`, `resolve()` |
| **aule-cache** | Local cache, activation, hook execution | `CacheManager`, `ActivationState`, `execute_hook()` |
| **aule-cli** | CLI binary wrapping all crates | `Commands` enum, subcommand handlers |

### Local Cache

Installed skills live in `~/.skills/` (configurable via `SKILL_HOME`):

```
~/.skills/
  config.json              # Registry URL, auth, policies
  metadata/
    index.json             # Metadata index for all installed skills
  artifacts/
    my-skill/
      1.0.0/
        skill.yaml         # Manifest
        content/           # Skill content
        .integrity         # SHA-256 verification hash
  activation/
    claude-code.json       # Which skills are active per runtime
    codex.json
```

## CLI Reference

All commands support `--json` for machine-readable output.

```
skill init [--name <NAME>]                Initialize a new skill package
skill validate [--path <PATH>]            Validate manifest and contract
skill build [--path <PATH>] [--target <RUNTIME>] [--output <DIR>]
                                          Generate adapter output
skill migrate [--path <PATH>]             Migrate a v0.1.0 manifest to v0.2.0
skill install <SOURCE> [--ref <GIT_REF>] [--version <CONSTRAINT>] [--target <RUNTIME>]
                                          Install from path, git, or registry
skill activate <NAME> [--target <RUNTIME>]
                                          Bind an installed skill to a runtime
skill list [--installed] [--active]       List skills
skill login [--registry <URL>]            Authenticate with registry (GitHub OAuth)
skill logout                              Remove auth token
skill publish [--path <PATH>] [--ref <GIT_REF>]
                                          Publish a skill to the registry
skill search <QUERY> [--runtime <RUNTIME>] [--limit <N>]
                                          Search the registry
```

## Configuration

User configuration lives in `~/.skills/config.json`:

```json
{
  "default_targets": ["claude-code"],
  "policy": {
    "allow": ["@trusted-publisher/*"],
    "block": ["@blocked-publisher/*"]
  },
  "registry_url": "https://aule.dev"
}
```

### Policy Model

| Policy | Effect |
|--------|--------|
| `allow` | Only skills matching these patterns can be installed |
| `block` | Skills matching these patterns are rejected |
| No policy | All installs are permitted |

## Development

### Prerequisites

- Rust 1.70+ (edition 2021)
- Cargo

### Build and test

```bash
cargo build                # Build all crates
cargo test                 # Run all tests (~122 tests)
cargo test -p aule-schema  # Test a single crate
```

### Run the CLI locally

```bash
cargo run -p aule-cli -- init --name test-skill
cargo run -p aule-cli -- validate --path examples/skill-init/
cargo run -p aule-cli -- build --path examples/skill-init/ --target claude-code
```

### Validation gate

The real skill validation test generates adapter output for all example skills and asserts the output matches the hand-written files in `.claude/` and `.codex/`:

```bash
cargo test -p aule-adapter --test real_skills_test
```

This is the integration gate — if adapter generation changes, this test catches it.

### Project structure

```
aule/
├── Cargo.toml             # Workspace definition
├── crates/
│   ├── aule-schema/       # Protocol types and validation
│   ├── aule-adapter/      # Runtime adapter generation
│   ├── aule-resolver/     # Version resolution (local, git, registry)
│   ├── aule-cache/        # Local cache and activation
│   └── aule-cli/          # CLI binary
├── examples/              # Example skill packages
│   ├── skill-init/              # v0.1.0 examples
│   ├── skill-validate/
│   ├── skill-build/
│   ├── skill-publish/
│   ├── skill-develop/
│   ├── skill-scout/
│   └── api-contract-tester/     # v0.2.0 multi-skill + tools example
├── platform/              # Registry web application (Next.js + Supabase)
├── .claude/skills/        # Generated Claude Code adapter output
└── .codex/skills/         # Generated Codex adapter output
```

## Roadmap

### Done

- [x] Manifest schema v0.1.0 — single-skill packages with prose content
- [x] Manifest schema v0.2.0 — multi-skill packages, executable tools, lifecycle hooks
- [x] Contract model (inputs, outputs, permissions, determinism)
- [x] Permission vocabulary with policy enforcement
- [x] Adapter generator (v0.1.0: SKILL.md; v0.2.0: SKILL.md + wrapper scripts + tool docs + file bundling)
- [x] Multi-source resolver (local path, git URL, registry identifier)
- [x] Local cache with integrity verification (`~/.skills/`)
- [x] Lifecycle hooks (onInstall, onActivate, onUninstall)
- [x] CLI with full lifecycle: `init`, `validate`, `build`, `install`, `activate`, `list`, `migrate`
- [x] Registry commands: `login`, `logout`, `publish`, `search`
- [x] Seven example skills (six v0.1.0, one v0.2.0 with tools and hooks)

### In Progress

- [ ] Hosted skill registry ([aule.dev](https://aule.dev))
- [ ] Skill indexing pipeline (fetch manifests from GitHub)
- [ ] Web UI for browsing and discovering skills
- [ ] Additional runtime adapters

## Contributing

Contributions are welcome. Here's how to get started:

1. **Fork and clone** the repository
2. **Create a branch** for your change
3. **Write tests** — the project has ~122 tests and additions should maintain coverage
4. **Run `cargo test`** to verify everything passes
5. **Submit a pull request** with a clear description of what and why

### Areas where help is appreciated

- **New runtime adapters** — adding support for more AI coding agents
- **Skill contributions** — building and publishing useful skills
- **Registry features** — improving the web platform and API
- **Documentation** — guides, tutorials, and examples

## License

[MIT](LICENSE)
