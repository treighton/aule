## MODIFIED Requirements

### Requirement: Schema version routing
The manifest parser SHALL inspect the `schemaVersion` field and select the appropriate parsing path. `"0.1.0"` SHALL use the existing schema. `"0.2.0"` SHALL use the new schema with `skills`, `tools`, `hooks`, and `files` fields.

#### Scenario: v0.1.0 manifest parsed unchanged
- **WHEN** a manifest with `schemaVersion: "0.1.0"` is loaded
- **THEN** the parser SHALL parse it using the v0.1.0 schema (with `content` and `contract` fields)
- **THEN** no errors or warnings SHALL be emitted about missing v0.2.0 fields

#### Scenario: v0.2.0 manifest parsed
- **WHEN** a manifest with `schemaVersion: "0.2.0"` is loaded
- **THEN** the parser SHALL expect `files`, `skills`, and allow optional `tools` and `hooks`
- **THEN** the parser SHALL reject the manifest if it contains v0.1.0 fields (`content`, `contract`) instead of v0.2.0 fields

#### Scenario: Unknown schema version
- **WHEN** a manifest with `schemaVersion: "0.3.0"` is loaded
- **THEN** the parser SHALL reject the manifest with an error indicating unsupported schema version

### Requirement: Files field replaces content
In v0.2.0 manifests, the `content` field SHALL NOT be present. The `files` field SHALL be a list of glob strings declaring all files bundled with the package.

#### Scenario: Files as glob list
- **WHEN** a v0.2.0 manifest declares `files: ["content/**", "logic/**"]`
- **THEN** the parser SHALL store the list of glob patterns for use by validation and the adapter

#### Scenario: Empty files list
- **WHEN** a v0.2.0 manifest declares `files: []`
- **THEN** validation SHALL emit a warning (a package with no files is unusual but not invalid)

### Requirement: Skills field replaces contract
In v0.2.0 manifests, the `contract` field SHALL NOT be present. The `skills` field SHALL be a map of skill definitions, each containing the interface fields previously in `contract` plus `entrypoint` and `description`.

#### Scenario: Skill definition has all interface fields
- **WHEN** a skill definition includes `version`, `inputs`, `outputs`, `permissions`, `determinism`
- **THEN** the parser SHALL validate each field with the same rules as v0.1.0 `contract` fields

#### Scenario: v0.2.0 manifest with contract field
- **WHEN** a v0.2.0 manifest contains a `contract` field
- **THEN** the parser SHALL reject the manifest with an error indicating `contract` is not valid in v0.2.0 — use `skills` instead

### Requirement: Top-level fields unchanged
The following top-level fields SHALL remain unchanged between v0.1.0 and v0.2.0: `name`, `description`, `version`, `identity`, `adapters`, `metadata`, `dependencies`, `extensions`.

#### Scenario: Shared fields work in both versions
- **WHEN** a v0.2.0 manifest includes `name`, `metadata.tags`, `dependencies.tools`, and `adapters.claude-code`
- **THEN** the parser SHALL handle them identically to v0.1.0
