## MODIFIED Requirements

### Requirement: Adapter handles v0.2.0 manifest
The adapter generator SHALL accept v0.2.0 manifests and generate output for each skill in the `skills` map, producing separate SKILL.md files per skill.

#### Scenario: Single skill in v0.2.0 manifest
- **WHEN** `skill build` is run on a v0.2.0 manifest with one skill
- **THEN** the adapter SHALL generate one SKILL.md with frontmatter derived from the skill definition and body from the skill's entrypoint

#### Scenario: Multiple skills in v0.2.0 manifest
- **WHEN** `skill build` is run on a v0.2.0 manifest with three skills
- **THEN** the adapter SHALL generate three SKILL.md files, each in its own subdirectory under the runtime's skill directory

### Requirement: Adapter generates tool wrappers
When a v0.2.0 manifest declares tools, the adapter SHALL generate wrapper scripts and tool documentation as specified in the `wrapper-script-generation` spec.

#### Scenario: Build with tools
- **WHEN** `skill build` runs on a manifest with two tools
- **THEN** the output directory SHALL contain a `tools/` subdirectory with two executable wrapper scripts and the tool documentation appended to each SKILL.md

#### Scenario: Build without tools
- **WHEN** `skill build` runs on a v0.2.0 manifest with no `tools` field
- **THEN** the adapter SHALL generate SKILL.md files as normal with no `tools/` directory and no `## Tools` section

### Requirement: Adapter copies included files
The adapter SHALL resolve all glob patterns in the `files` list and copy matched files into the generated output directory, preserving relative paths.

#### Scenario: Include glob resolution
- **WHEN** `files: ["logic/**"]` matches 5 files in the source directory
- **THEN** all 5 files SHALL appear in the generated output at their original relative paths

#### Scenario: Overlapping globs
- **WHEN** `files: ["logic/**", "logic/tools/*.ts"]` both match `logic/tools/generate.ts`
- **THEN** the file SHALL be copied once (no duplicate errors)

### Requirement: Adapter backward compatibility
The adapter SHALL continue to accept v0.1.0 manifests and generate output using the existing v0.1.0 logic. No v0.1.0 behavior SHALL change.

#### Scenario: v0.1.0 manifest builds unchanged
- **WHEN** `skill build` runs on an existing v0.1.0 manifest
- **THEN** the output SHALL be byte-identical to previous builds

### Requirement: Frontmatter mapping for v0.2.0
The adapter SHALL generate frontmatter for each skill using the skill's own `description` and `version`, and the package's `metadata` (author, license). The `compatibility` note SHALL be derived from `dependencies.tools` as in v0.1.0.

#### Scenario: Frontmatter content
- **WHEN** the adapter generates SKILL.md for a skill named `contract-tester` in package `api-testing-suite` with `metadata.author: "aule"` and `metadata.license: "MIT"`
- **THEN** the frontmatter SHALL include `name: contract-tester`, `description:` from the skill definition, `license: MIT`, and `metadata.author: aule`

### Requirement: Per-skill command generation
When a skill in a v0.2.0 manifest declares `commands`, the adapter SHALL generate command files in the runtime's command directory, namespaced to avoid collisions between skills in the same package.

#### Scenario: Command namespacing
- **WHEN** skill `contract-tester` in package `api-testing-suite` declares command `test-api`
- **THEN** the adapter SHALL generate the command in the runtime's command directory with appropriate namespacing (e.g., derived from the skill name)
