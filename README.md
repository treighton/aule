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
                    skill.yaml + content/skill.md
                               │
                               ▼
                    ┌─────────────────────┐
                    │    aule-schema      │  Parse & validate manifest,
                    │                     │  contract, permissions
                    └─────────┬───────────┘
                              │
                              ▼
                    ┌─────────────────────┐
                    │    aule-adapter     │  Generate runtime-specific
                    │                     │  SKILL.md with frontmatter
                    └─────────┬───────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
     .claude/skills/   .codex/skills/    (future runtimes)
      SKILL.md          SKILL.md
```

The key design principle: **your skill body passes through unchanged**. Only the YAML frontmatter is adapted per runtime. This preserves author intent and keeps diffs readable.

## Skill Manifest

A `skill.yaml` declares everything about a skill:

```yaml
schemaVersion: "0.1.0"
name: "my-skill"
description: "A brief description for discovery and triggering"
version: "1.0"

content:
  skill: "content/skill.md"

contract:
  version: "1.0.0"
  inputs: "prompt"            # or JSON Schema
  outputs: "prompt"           # or JSON Schema
  permissions:
    - "filesystem.read"
    - "process.spawn"
  determinism: "probabilistic" # deterministic | bounded | probabilistic

adapters:
  claude-code:
    enabled: true
  codex:
    enabled: true

metadata:
  author: "your-name"
  license: "MIT"

dependencies:
  tools:
    - name: "external-cli"
      version: "1.0"
  skills:
    - name: "base-skill"
      version: "^1.0"
```

### Contract

The contract is the versioned interface. It declares:

| Field | Purpose |
|-------|---------|
| `inputs` / `outputs` | `"prompt"` for unstructured text, or a JSON Schema for structured data |
| `permissions` | Capabilities the skill requires (filesystem, network, process scopes) |
| `determinism` | Whether output is deterministic, bounded, or probabilistic |
| `errors` | Error codes the skill can produce |
| `behavior.timeout_ms` | Maximum execution time |

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
| **aule-schema** | Protocol types and validation | `Manifest`, `Contract`, `Permission`, `Envelope` |
| **aule-adapter** | Adapter generation | `RuntimeTarget`, `GeneratedFile`, `generate()` |
| **aule-resolver** | Skill resolution from multiple sources | `ResolveRequest`, `ResolvePlan`, `resolve()` |
| **aule-cache** | Local cache and activation state | `Cache`, `Artifact`, `ActivationState` |
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
cargo test                 # Run all tests (~97 tests)
cargo test -p aule-schema  # Test a single crate
```

### Run the CLI locally

```bash
cargo run -p aule-cli -- init --name test-skill
cargo run -p aule-cli -- validate --path skills/openspec-explore/
cargo run -p aule-cli -- build --path skills/openspec-explore/ --target claude-code
```

### Validation gate

The real skill validation test generates adapter output for all four included skills and asserts the output matches the hand-written files in `.claude/` and `.codex/`:

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
├── skills/                # Example skill packages
│   ├── openspec-apply-change/
│   ├── openspec-archive-change/
│   ├── openspec-explore/
│   └── openspec-propose/
├── platform/              # Registry web application (Next.js + Supabase)
├── .claude/skills/        # Generated Claude Code adapter output
└── .codex/skills/         # Generated Codex adapter output
```

## Roadmap

### Done

- [x] Manifest schema and validation (`skill.yaml`)
- [x] Contract model (inputs, outputs, permissions, determinism)
- [x] Permission vocabulary with policy enforcement
- [x] Adapter generator (manifest to runtime-specific SKILL.md)
- [x] Multi-source resolver (local path, git URL, registry identifier)
- [x] Local cache with integrity verification (`~/.skills/`)
- [x] CLI with full lifecycle: `init`, `validate`, `build`, `install`, `activate`, `list`
- [x] Registry commands: `login`, `logout`, `publish`, `search`
- [x] Four example skills (OpenSpec workflow)

### In Progress

- [ ] Hosted skill registry ([aule.dev](https://aule.dev))
- [ ] Skill indexing pipeline (fetch manifests from GitHub)
- [ ] Web UI for browsing and discovering skills
- [ ] Additional runtime adapters

## Contributing

Contributions are welcome. Here's how to get started:

1. **Fork and clone** the repository
2. **Create a branch** for your change
3. **Write tests** — the project has ~97 tests and additions should maintain coverage
4. **Run `cargo test`** to verify everything passes
5. **Submit a pull request** with a clear description of what and why

### Areas where help is appreciated

- **New runtime adapters** — adding support for more AI coding agents
- **Skill contributions** — building and publishing useful skills
- **Registry features** — improving the web platform and API
- **Documentation** — guides, tutorials, and examples

## License

[MIT](LICENSE)
