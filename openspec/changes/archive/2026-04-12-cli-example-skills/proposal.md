## Why

The `skills/` directory currently contains clones of the 4 OpenSpec skills as example skill packages. These serve as both documentation of the skill format and as golden-file test fixtures for the adapter generator. However, they don't demonstrate the Aulë CLI itself, and the directory name `skills/` is ambiguous — it looks like these are "the skills that ship with the tool" rather than example packages.

We need example skills that dogfood our own CLI, demonstrate the full manifest surface area (including skill-to-skill dependencies), and serve as better test fixtures for the adapter generator.

## What Changes

- Rename `skills/` → `examples/` to clarify intent
- Remove the 4 OpenSpec skill packages (`openspec-explore`, `openspec-propose`, `openspec-apply-change`, `openspec-archive-change`) from the examples directory
- Remove corresponding adapter output from `.claude/skills/openspec-*` and `.codex/skills/openspec-*`
- Create 6 new example skills that help agents use the `skill` CLI:
  - **`skill-init`** — simple wrapper around `skill init` (deterministic, no deps)
  - **`skill-validate`** — simple wrapper around `skill validate` (deterministic, no deps)
  - **`skill-build`** — simple wrapper around `skill build` (deterministic, tool dep: `skill`)
  - **`skill-publish`** — simple wrapper around `skill publish` (bounded, tool dep: `skill`)
  - **`skill-develop`** — composer skill with `deps.skills` referencing the 4 simple wrappers; orchestrates a research → plan → implement → validate loop; reads `docs/authoring-skills.md` at runtime for schema knowledge
  - **`skill-scout`** — autonomous consumer skill for discovering, evaluating, installing, and running skills; exercises the full manifest surface area (`identity`, `tags`, `extensions`, `contract.errors`, `contract.behavior`); configurable autonomy mode (supervised: 4 permission gates vs. autonomous: 1 gate)
- Update `real_skills_test.rs` to generate and compare the 6 new skills instead of the 4 OpenSpec skills
- Update path references in `CLAUDE.md`, `README.md`, `CONTRIBUTING.md`

**Note:** The adapter generator (`build_skill_frontmatter`) currently only emits `name`, `description`, `license`, `compatibility`, and `metadata` in frontmatter. Fields like `identity`, `tags`, `extensions`, `contract.errors`, and `contract.behavior` exist in the schema but are not yet surfaced in adapter output. This change uses those fields in manifests (proving schema parsing works) but does NOT extend the adapter generator — that's a separate future change.

## Capabilities

### New Capabilities
- `example-skill-content`: Content and manifests for the 6 new example skills, covering the full spectrum from minimal CLI wrappers to complex autonomous workflows
- `example-test-fixtures`: Golden-file test fixtures using the new example skills, replacing the OpenSpec-based fixtures

### Modified Capabilities
- `adapter-generator`: Path change from `skills/` to `examples/` in the real_skills_test — no spec-level behavior change, just test infrastructure

## Impact

- **Test suite**: `real_skills_test.rs` changes from 4 test functions to 6; path changes from `skills/` to `examples/`
- **Adapter output**: `.claude/skills/` and `.codex/skills/` directories lose the `openspec-*` entries, gain 6 new entries
- **Documentation**: `CLAUDE.md`, `README.md`, `CONTRIBUTING.md` path references update
- **OpenSpec change docs**: Historical references to `skills/openspec-explore/` in prior change artifacts are left as-is (they're historical records)
- **No Rust code changes to the adapter generator** — only test file path updates
