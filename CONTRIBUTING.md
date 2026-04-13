# Contributing to Aule

Thank you for your interest in contributing to Aule. This document explains how to get set up, how the project is organized, and how to submit changes.

## Getting Started

### Prerequisites

- **Rust 1.70+** (install via [rustup](https://rustup.rs/))
- **Git**
- **Node.js 20+** and **npm** (only for the `platform/` registry app)

### Clone and build

```bash
git clone https://github.com/treightonmauldin/aule.git
cd aule
cargo build
cargo test
```

All 97 tests should pass. If they don't, please open an issue.

## Project Layout

```
crates/
  aule-schema/       Protocol types and validation
  aule-adapter/      Runtime adapter generation
  aule-resolver/     Multi-source version resolution
  aule-cache/        Local cache and activation state
  aule-cli/          CLI binary (thin wrapper)
skills/              Example skill packages
platform/            Registry web app (Next.js + Supabase)
docs/                Documentation
```

Library crates contain the logic. The CLI binary in `aule-cli` is a thin wrapper that maps subcommands to library calls. When adding features, put the logic in the appropriate library crate and expose it through the CLI.

## Making Changes

### 1. Create a branch

```bash
git checkout -b my-change
```

### 2. Write your code

Follow the existing patterns in the crate you're modifying. Key conventions:

- **Error handling**: Use `thiserror` for error types. Every crate has its own error enum.
- **Serialization**: Use `serde` with `serde_yaml` for manifests and `serde_json` for config/API.
- **Testing**: Write tests alongside your code. See the testing section below.
- **Output**: CLI commands support `--json` for machine-readable output. Maintain this for new commands.

### 3. Test

```bash
# Run all tests
cargo test

# Run tests for the crate you changed
cargo test -p aule-schema

# Run the integration gate (checks adapter output against reference files)
cargo test -p aule-adapter --test real_skills_test
```

### 4. Submit a pull request

Push your branch and open a PR. Include:

- **What** you changed
- **Why** — the problem or improvement
- **Testing** — what you tested and how

## Testing

### Test organization

Each crate has its own tests:

| Crate | Tests | Focus |
|-------|-------|-------|
| `aule-schema` | 36 | Parsing, validation, edge cases |
| `aule-adapter` | 8 | Generation correctness, real skill validation |
| `aule-resolver` | 18 | Resolution from each source, policy enforcement |
| `aule-cache` | 17 | Install, activate, integrity, indexing |
| `aule-cli` | 14 | End-to-end CLI integration tests |

### The validation gate

The most important test is `crates/aule-adapter/tests/real_skills_test.rs`. It generates adapter output for all four included skills and asserts byte-for-byte equality with the committed files in `.claude/` and `.codex/`.

If you change adapter generation logic, update the reference files:

```bash
# Regenerate reference output
cargo run -p aule-cli -- build --path skills/openspec-explore/ --target claude-code
cargo run -p aule-cli -- build --path skills/openspec-explore/ --target codex
# Repeat for all four skills, then commit the updated output
```

### Writing tests

- Use `tempfile::TempDir` for tests that create files
- Use `assert_cmd` for CLI integration tests
- Test both success and failure paths
- Test JSON output mode for CLI commands

## Areas of Contribution

### New runtime adapters

Adding support for a new AI coding agent:

1. Define the runtime target in `aule-adapter/src/target.rs`
2. Add the target name to `aule-schema`'s known adapters
3. Add activation support in `aule-cache`
4. Write tests and generate reference output

See [docs/architecture.md](docs/architecture.md#adding-a-new-runtime-target) for details.

### Skills

Build and publish useful skills. Good candidates:

- Code review and analysis
- Testing workflows
- Documentation generation
- Refactoring patterns
- Security scanning
- Project scaffolding

### Registry and platform

The `platform/` directory contains the Next.js registry application. Contributions here include:

- Search and browse UI
- Skill detail pages
- Publisher profiles
- API improvements

### Documentation

- Tutorials and guides
- Example skills with walkthrough explanations
- Integration guides for specific agents

## Code Style

- Follow standard Rust conventions (`cargo fmt`, `cargo clippy`)
- Keep functions focused — prefer small functions with clear names
- Error messages should be actionable ("expected schema version 0.1.0, got 0.2.0")
- Comments explain *why*, not *what*

## Questions?

Open an issue for questions, feature proposals, or bug reports.
