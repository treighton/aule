## ADDED Requirements

### Requirement: Golden-file test functions for all 6 example skills
The `real_skills_test.rs` file SHALL contain one test function per example skill, each using the `generate_and_compare` pattern. The `generate_and_compare` helper SHALL read from `examples/` (not `skills/`).

#### Scenario: Path updated to examples directory
- **WHEN** `generate_and_compare` resolves the skill source path
- **THEN** it SHALL use `root.join("examples").join(skill_name)` instead of `root.join("skills").join(skill_name)`

#### Scenario: Test for each example skill
- **WHEN** the test suite runs
- **THEN** there SHALL be 6 test functions: `skill_init_matches`, `skill_validate_matches`, `skill_build_matches`, `skill_publish_matches`, `skill_develop_matches`, and `skill_scout_matches`

#### Scenario: Generated output matches checked-in output
- **WHEN** any test function runs `generate_and_compare` for a skill
- **THEN** the generated Claude Code SKILL.md SHALL be byte-identical to the checked-in file at `.claude/skills/<name>/SKILL.md`, and the generated Codex SKILL.md SHALL be byte-identical to `.codex/skills/<name>/SKILL.md`

### Requirement: Checked-in adapter output for all 6 example skills
The repository SHALL contain pre-generated adapter output in `.claude/skills/` and `.codex/skills/` for all 6 example skills. Old OpenSpec adapter output SHALL be removed.

#### Scenario: Old adapter output removed
- **WHEN** the change is complete
- **THEN** `.claude/skills/openspec-explore/`, `.claude/skills/openspec-propose/`, `.claude/skills/openspec-apply-change/`, `.claude/skills/openspec-archive-change/` and their `.codex/` equivalents SHALL NOT exist

#### Scenario: New adapter output present
- **WHEN** the change is complete
- **THEN** `.claude/skills/skill-init/SKILL.md`, `.claude/skills/skill-validate/SKILL.md`, `.claude/skills/skill-build/SKILL.md`, `.claude/skills/skill-publish/SKILL.md`, `.claude/skills/skill-develop/SKILL.md`, `.claude/skills/skill-scout/SKILL.md` and their `.codex/` equivalents SHALL exist

### Requirement: Documentation references updated
All documentation files (`CLAUDE.md`, `README.md`, `CONTRIBUTING.md`) SHALL reference `examples/` instead of `skills/` for the example skill packages directory.

#### Scenario: No stale references
- **WHEN** all markdown files in the repo root and `docs/` are searched for the pattern `skills/openspec-`
- **THEN** no matches SHALL be found outside of `openspec/changes/` (historical change artifacts are exempt)

#### Scenario: Example commands use new path
- **WHEN** `README.md` or `CONTRIBUTING.md` contain example CLI commands referencing example skills
- **THEN** those commands SHALL use `examples/` as the path prefix (e.g., `skill validate --path examples/skill-init/`)
