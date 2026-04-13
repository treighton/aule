## ADDED Requirements

### Requirement: Skills map in manifest
The manifest SHALL support a top-level `skills` map where each key is a skill name (kebab-case) and each value is a skill definition containing: `description`, `entrypoint`, `version`, `inputs`, `outputs`, `permissions`, `determinism`, and optional `errors`, `behavior`, and `commands`.

#### Scenario: Single skill package
- **WHEN** a manifest declares one entry in `skills` with all required fields
- **THEN** the parser SHALL accept the manifest and produce a single skill definition

#### Scenario: Multi-skill package
- **WHEN** a manifest declares three entries in `skills`, each with distinct entrypoints, permissions, and interfaces
- **THEN** the parser SHALL accept the manifest and produce three independent skill definitions

#### Scenario: Skill name collision
- **WHEN** a manifest declares two skills with the same key name
- **THEN** YAML parsing SHALL use the last value (standard YAML behavior) — the parser MAY emit a warning

### Requirement: Per-skill entrypoint
Each skill definition SHALL include an `entrypoint` field pointing to a Markdown file relative to the manifest directory. This file contains the skill's prose instructions.

#### Scenario: Valid entrypoint
- **WHEN** a skill declares `entrypoint: "content/contract-tester.md"` and the file exists
- **THEN** validation SHALL pass and the adapter SHALL use this file as the skill body

#### Scenario: Missing entrypoint
- **WHEN** a skill declares an entrypoint that does not exist on disk
- **THEN** validation SHALL fail with an error identifying the missing file

### Requirement: Per-skill interface
Each skill definition SHALL declare its own `inputs`, `outputs`, `permissions`, `determinism`, and `version`. These fields have the same semantics as the v0.1.0 `contract` fields.

#### Scenario: Skills with different permissions
- **WHEN** a package contains skill A with `permissions: ["filesystem.read"]` and skill B with `permissions: ["filesystem.read", "network.external"]`
- **THEN** the parser SHALL store distinct permission sets for each skill

#### Scenario: Skills with different determinism levels
- **WHEN** a package contains a deterministic linter skill and a probabilistic diagnosis skill
- **THEN** the adapter SHALL reflect the correct determinism level in each generated SKILL.md

### Requirement: Per-skill commands
Each skill definition MAY include a `commands` map where each key is a command name and each value is a path to a command Markdown file.

#### Scenario: Skill with commands
- **WHEN** a skill declares `commands: { test-api: "content/commands/test-api.md" }`
- **THEN** the adapter SHALL generate the command file in the runtime's command directory, namespaced to the skill

#### Scenario: Skill without commands
- **WHEN** a skill omits the `commands` field
- **THEN** the parser SHALL accept the skill with no commands

### Requirement: Adapter generates per-skill output
The adapter SHALL generate a separate SKILL.md for each skill in the package, each placed in its own directory under the runtime's skill directory.

#### Scenario: Multi-skill adapter output
- **WHEN** `skill build` runs on a package with skills `contract-tester` and `spec-linter`
- **THEN** the adapter SHALL generate `.claude/skills/contract-tester/SKILL.md` and `.claude/skills/spec-linter/SKILL.md`
- **THEN** each SKILL.md SHALL contain frontmatter derived from its skill definition and the skill body from its entrypoint

### Requirement: Package-level identity
The manifest's top-level `name`, `version`, `description`, and `metadata` describe the package. Individual skills inherit the package metadata but override with their own `description` and `version`.

#### Scenario: Skill version vs package version
- **WHEN** a package has `version: "2.0.0"` and a skill declares `version: "1.0.0"` for its interface
- **THEN** the skill's interface version is `"1.0.0"` and the package version is `"2.0.0"` — they are independent
