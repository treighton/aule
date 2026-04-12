## ADDED Requirements

### Requirement: Manifest file format and location
A skill package SHALL contain a `skill.yaml` file at the package root. The manifest SHALL be valid YAML and SHALL validate against the Skill Manifest JSON Schema (`manifest.schema.json`). The manifest SHALL declare `schemaVersion: "0.1.0"` as a required top-level field.

#### Scenario: Valid manifest is accepted
- **WHEN** a `skill.yaml` file contains all required fields and valid YAML syntax
- **THEN** the manifest parser SHALL return a typed ManifestDocument object with no validation errors

#### Scenario: Missing manifest file
- **WHEN** a skill package directory does not contain a `skill.yaml` file
- **THEN** the manifest parser SHALL return an error indicating the manifest is missing

#### Scenario: Invalid YAML syntax
- **WHEN** a `skill.yaml` file contains malformed YAML
- **THEN** the manifest parser SHALL return a parse error with the line/column of the syntax issue

### Requirement: Required core fields
The manifest SHALL require the following top-level fields: `schemaVersion` (string, locked to `"0.1.0"`), `name` (kebab-case string, 1-100 chars), `description` (string, 1-500 chars), `version` (semver string), `content` (object defining skill content paths), and `contract` (object or reference to contract definition).

#### Scenario: All required fields present
- **WHEN** a manifest contains `schemaVersion`, `name`, `description`, `version`, `content`, and `contract`
- **THEN** validation SHALL pass with no errors on the required fields check

#### Scenario: Missing required field
- **WHEN** a manifest omits any required field (e.g., `name` is absent)
- **THEN** validation SHALL fail with an error identifying the missing field by name

#### Scenario: Name format enforcement
- **WHEN** a manifest `name` field contains characters other than lowercase alphanumeric and hyphens, or exceeds 100 characters
- **THEN** validation SHALL fail with a format error on the `name` field

### Requirement: Optional identity field for future resolution
The manifest SHALL support an optional `identity` field (string) in domain/path format (e.g., `skills.acme.dev/workflow/explore`). In v0 the resolver SHALL NOT require this field. When present, it SHALL be validated as a well-formed domain/path string.

#### Scenario: Identity field absent in v0
- **WHEN** a manifest omits the `identity` field
- **THEN** validation SHALL pass and the resolver SHALL use the `name` field as the local identifier

#### Scenario: Identity field present and well-formed
- **WHEN** a manifest includes `identity: "skills.acme.dev/workflow/explore"`
- **THEN** validation SHALL pass and the identity SHALL be stored in the parsed manifest

#### Scenario: Identity field malformed
- **WHEN** a manifest includes an `identity` value that is not a valid domain/path string (e.g., contains spaces or no domain component)
- **THEN** validation SHALL fail with a format error on the `identity` field

### Requirement: Content paths definition
The manifest `content` object SHALL contain a `skill` field (string path to the primary skill markdown file, relative to package root). It MAY contain a `commands` field (object mapping command names to relative file paths).

#### Scenario: Skill content path resolves
- **WHEN** a manifest declares `content.skill: "content/skill.md"` and that file exists in the package
- **THEN** the content reference SHALL be considered valid

#### Scenario: Skill content path missing
- **WHEN** a manifest declares `content.skill: "content/skill.md"` but the file does not exist
- **THEN** validation SHALL fail with a file-not-found error referencing the path

#### Scenario: Commands mapping
- **WHEN** a manifest declares `content.commands: { explore: "content/commands/explore.md", propose: "content/commands/propose.md" }`
- **THEN** each command path SHALL be validated for existence and the command names SHALL be available for adapter generation

### Requirement: Adapter targets declaration
The manifest SHALL contain an `adapters` object mapping runtime target identifiers to adapter configuration objects. v0 SHALL support `claude-code` and `codex` as target identifiers. Each adapter config object SHALL contain at minimum a `enabled` boolean field.

#### Scenario: Both adapters enabled
- **WHEN** a manifest declares `adapters: { claude-code: { enabled: true }, codex: { enabled: true } }`
- **THEN** the adapter generator SHALL produce output for both Claude Code and Codex

#### Scenario: One adapter disabled
- **WHEN** a manifest declares `adapters: { claude-code: { enabled: true }, codex: { enabled: false } }`
- **THEN** the adapter generator SHALL produce output only for Claude Code

#### Scenario: Unknown adapter target
- **WHEN** a manifest declares an adapter target not in the supported set (e.g., `adapters: { unknown-runtime: { enabled: true } }`)
- **THEN** validation SHALL emit a warning (not an error) and skip the unknown target during generation

### Requirement: Dependencies declaration
The manifest MAY contain a `dependencies` object with two sub-fields: `skills` (array of skill name/version constraint pairs for skill-to-skill dependencies) and `tools` (array of external tool dependency declarations with `name` and optional `version` fields). In v0, `skills` dependencies SHALL be recorded but not resolved. `tools` dependencies SHALL be included in generated adapter frontmatter where supported.

#### Scenario: External tool dependency declared
- **WHEN** a manifest declares `dependencies.tools: [{ name: "openspec", version: ">=1.0.0" }]`
- **THEN** the parsed manifest SHALL include the tool dependency and adapter generation SHALL include it in frontmatter `compatibility` fields where the runtime supports it

#### Scenario: No dependencies
- **WHEN** a manifest omits the `dependencies` field entirely
- **THEN** validation SHALL pass and the skill SHALL be treated as having no dependencies

### Requirement: Extension namespaces
The manifest MAY contain an `extensions` object with vendor/org-namespaced sub-objects (e.g., `extensions.vendor.claude`, `extensions.org.acme`). Extension fields SHALL NOT be validated by the core schema beyond verifying they are objects. Core tooling SHALL ignore extension fields.

#### Scenario: Extension namespace present
- **WHEN** a manifest includes `extensions: { vendor: { claude: { maxTokens: 4096 } } }`
- **THEN** validation SHALL pass and the extension data SHALL be preserved in the parsed manifest but not interpreted by core tooling

#### Scenario: Extension at wrong nesting level
- **WHEN** a manifest includes extension-like fields at the top level (e.g., `maxTokens: 4096` outside of `extensions`)
- **THEN** validation SHALL fail with an unexpected-field error

### Requirement: Metadata fields
The manifest MAY contain a `metadata` object with fields: `author` (string), `license` (string, SPDX identifier), `homepage` (URL string), `repository` (URL string), `tags` (array of strings, max 10). All metadata fields are optional.

#### Scenario: Full metadata present
- **WHEN** a manifest includes `metadata: { author: "openspec", license: "MIT", tags: ["workflow", "explore"] }`
- **THEN** validation SHALL pass and metadata SHALL be available for adapter generation and registry indexing

#### Scenario: Tags exceed limit
- **WHEN** a manifest includes more than 10 tags
- **THEN** validation SHALL fail with an error indicating the tag count limit
