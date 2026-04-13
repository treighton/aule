## ADDED Requirements

### Requirement: Simple CLI wrapper skills
The system SHALL include 4 simple example skills (`skill-init`, `skill-validate`, `skill-build`, `skill-publish`) that each wrap a single `skill` CLI command. Each skill SHALL have a `skill.yaml` manifest and a `content/skill.md` body. Each skill's content SHALL interactively guide the agent through running the corresponding CLI command, including asking for required inputs and displaying results.

#### Scenario: skill-init guides through scaffolding
- **WHEN** the `skill-init` skill is activated
- **THEN** the agent SHALL ask the user for a skill name, run `skill init --name <name>`, and display the scaffolded directory structure

#### Scenario: skill-validate reports errors with fix suggestions
- **WHEN** the `skill-validate` skill is activated with a path to a skill directory
- **THEN** the agent SHALL run `skill validate --path <path>`, parse the output, and for each validation error provide a specific suggestion for how to fix it

#### Scenario: skill-build generates adapter output
- **WHEN** the `skill-build` skill is activated
- **THEN** the agent SHALL ask for a target runtime (or default to all enabled), run `skill build`, and display the generated file paths and a preview of the frontmatter

#### Scenario: skill-publish handles authentication
- **WHEN** the `skill-publish` skill is activated and the user is not authenticated
- **THEN** the agent SHALL detect the auth failure, explain the issue, and offer to run `skill login` first

### Requirement: Composer skill with dependency declaration
The system SHALL include a `skill-develop` example skill that declares `deps.skills` referencing `skill-init`, `skill-validate`, `skill-build`, and `skill-publish`. The skill SHALL orchestrate a research → plan → implement → validate loop for creating new skills.

#### Scenario: Research phase reads schema documentation
- **WHEN** the `skill-develop` skill enters the research phase
- **THEN** the agent SHALL read `docs/authoring-skills.md` to learn the current manifest field reference, permission vocabulary, and determinism levels

#### Scenario: Plan phase designs manifest and content
- **WHEN** the research phase is complete
- **THEN** the agent SHALL propose a manifest structure (fields, permissions, adapters, dependencies) and an outline for the skill content, and ask the user to confirm or adjust before proceeding

#### Scenario: Validate phase loops on errors
- **WHEN** `skill validate` reports errors after implementation
- **THEN** the agent SHALL analyze each error, apply fixes to the manifest or content, and re-run validation until clean or until it needs user input to resolve an ambiguity

#### Scenario: Build phase verifies adapter output
- **WHEN** validation passes
- **THEN** the agent SHALL run `skill build` for all enabled adapter targets, read the generated SKILL.md files, and confirm the frontmatter and content look correct before offering to publish

### Requirement: Autonomous consumer skill with configurable gates
The system SHALL include a `skill-scout` example skill that autonomously discovers, evaluates, installs, activates, and runs skills from the registry. The skill SHALL support two gate modes configurable at activation time.

#### Scenario: Supervised mode with 4 permission gates
- **WHEN** the user selects supervised mode
- **THEN** the agent SHALL ask for permission before each of: (1) searching the registry, (2) evaluating a specific skill's contract, (3) installing the skill, (4) activating and running the skill

#### Scenario: Autonomous mode with 1 permission gate
- **WHEN** the user selects autonomous mode
- **THEN** the agent SHALL search and evaluate silently, then present a single prompt: "Found [skill] (permissions: [list]). Install, activate, and run it?"

#### Scenario: Permissions always shown before install
- **WHEN** the agent proposes installing a skill in either gate mode
- **THEN** the agent SHALL display the skill's declared permissions from its contract before requesting approval — installation SHALL NOT proceed without the user seeing the permission list

#### Scenario: Search with no results
- **WHEN** a registry search returns no matching skills
- **THEN** the agent SHALL report no results found and suggest alternative search terms or manual skill creation

### Requirement: Full manifest surface area coverage
The `skill-scout` manifest SHALL exercise every field the schema supports: `identity`, `contract.errors`, `contract.behavior.timeout_ms`, `metadata.tags`, `metadata.homepage`, `metadata.repository`, and `extensions`. The 4 simple skills SHALL use minimal manifests. `skill-develop` SHALL be the only skill using `deps.skills`.

#### Scenario: skill-scout manifest includes all optional fields
- **WHEN** the `skill-scout` skill.yaml is parsed by the schema crate
- **THEN** all fields SHALL parse successfully: `identity` as a valid domain/path, `contract.errors` as a list of code/description pairs, `contract.behavior.timeout_ms` as a positive integer, `metadata.tags` as a list of strings, `metadata.homepage` and `metadata.repository` as strings, and `extensions` as a nested map

#### Scenario: Simple skills use minimal manifests
- **WHEN** any of the 4 simple skill manifests are parsed
- **THEN** they SHALL contain only required fields (`schemaVersion`, `name`, `description`, `version`, `content`, `contract`, `adapters`) plus `metadata.author` and `metadata.license`

#### Scenario: skill-develop declares skill dependencies
- **WHEN** the `skill-develop` manifest is parsed
- **THEN** `dependencies.skills` SHALL contain entries for `skill-init`, `skill-validate`, `skill-build`, and `skill-publish`

### Requirement: Determinism level variety
The example skills SHALL collectively demonstrate all three determinism levels: `deterministic`, `bounded`, and `probabilistic`.

#### Scenario: Deterministic skills
- **WHEN** `skill-init`, `skill-validate`, or `skill-build` manifests are parsed
- **THEN** `contract.determinism` SHALL be `"deterministic"`

#### Scenario: Bounded skill
- **WHEN** the `skill-publish` manifest is parsed
- **THEN** `contract.determinism` SHALL be `"bounded"`

#### Scenario: Probabilistic skills
- **WHEN** `skill-develop` or `skill-scout` manifests are parsed
- **THEN** `contract.determinism` SHALL be `"probabilistic"`
