## ADDED Requirements

### Requirement: Contract definition location
A contract SHALL be defined either inline within the manifest's `contract` field or in a separate `contract.yaml` file referenced by the manifest. When defined inline, the `contract` field SHALL be an object conforming to the contract schema. When defined as a reference, the `contract` field SHALL be a string path relative to the package root.

#### Scenario: Inline contract
- **WHEN** a manifest declares `contract` as an object with valid contract fields
- **THEN** the contract parser SHALL extract the contract directly from the manifest

#### Scenario: Referenced contract file
- **WHEN** a manifest declares `contract: "contract.yaml"` and that file exists and contains valid contract YAML
- **THEN** the contract parser SHALL load and validate the external contract file

#### Scenario: Referenced contract file missing
- **WHEN** a manifest declares `contract: "contract.yaml"` but the file does not exist
- **THEN** validation SHALL fail with a file-not-found error

### Requirement: Contract version field
The contract SHALL contain a `version` field (semver string) independent of the skill's package version. The contract version SHALL change when input/output schemas, permissions, or determinism guarantees change in a breaking way.

#### Scenario: Contract version present and valid
- **WHEN** a contract includes `version: "1.0.0"`
- **THEN** validation SHALL pass for the version field

#### Scenario: Contract version missing
- **WHEN** a contract omits the `version` field
- **THEN** validation SHALL fail with a missing-field error

### Requirement: Input and output schema definitions
The contract SHALL contain `inputs` and `outputs` fields. Each field SHALL be either the string `"prompt"` (indicating unstructured text input/output) or a JSON Schema object defining structured data. v0 skills SHALL commonly use `"prompt"` for both.

#### Scenario: Prompt-based skill contract
- **WHEN** a contract declares `inputs: "prompt"` and `outputs: "prompt"`
- **THEN** validation SHALL pass and the contract SHALL be classified as a prompt-based skill

#### Scenario: Structured input schema
- **WHEN** a contract declares `inputs: { type: "object", properties: { query: { type: "string" } }, required: ["query"] }`
- **THEN** validation SHALL pass and the input schema SHALL be available for invocation envelope validation

#### Scenario: Invalid input schema
- **WHEN** a contract declares `inputs` as a value that is neither `"prompt"` nor a valid JSON Schema object
- **THEN** validation SHALL fail with a schema-format error

### Requirement: Permissions declaration in contract
The contract SHALL contain a `permissions` field (array of strings from the permission vocabulary). An empty array indicates the skill requires no special permissions.

#### Scenario: Permissions declared
- **WHEN** a contract declares `permissions: ["filesystem.read", "network.external"]`
- **THEN** validation SHALL pass and the permissions SHALL be available for policy evaluation

#### Scenario: Empty permissions
- **WHEN** a contract declares `permissions: []`
- **THEN** validation SHALL pass indicating a zero-permission skill

#### Scenario: Unknown permission string
- **WHEN** a contract declares a permission not in the v0 vocabulary (e.g., `"quantum.entangle"`)
- **THEN** validation SHALL emit a warning but SHALL NOT fail (forward-compatibility for vocabulary expansion)

### Requirement: Determinism bounds declaration
The contract SHALL contain a `determinism` field with one of three values: `"deterministic"` (same input always produces same output), `"bounded"` (output varies within documented bounds), or `"probabilistic"` (output varies, typical for LLM-based skills). Default SHALL be `"probabilistic"` if omitted.

#### Scenario: Determinism explicitly set
- **WHEN** a contract declares `determinism: "bounded"`
- **THEN** validation SHALL pass and the determinism bound SHALL be recorded

#### Scenario: Determinism omitted
- **WHEN** a contract omits the `determinism` field
- **THEN** the parser SHALL default to `"probabilistic"`

### Requirement: Error model declaration
The contract MAY contain an `errors` field defining expected error types as an array of objects with `code` (string) and `description` (string) fields. When omitted, the skill is assumed to use only the standard invocation envelope error format.

#### Scenario: Custom error types declared
- **WHEN** a contract declares `errors: [{ code: "CONTEXT_TOO_LARGE", description: "Input exceeds context window" }]`
- **THEN** validation SHALL pass and the error codes SHALL be available for invocation envelope error matching

#### Scenario: No custom errors
- **WHEN** a contract omits the `errors` field
- **THEN** validation SHALL pass and only standard envelope errors SHALL be expected

### Requirement: Optional behavioral metadata
The contract MAY contain a `behavior` object with optional fields: `latencyClass` (enum: `"fast"` | `"moderate"` | `"slow"`), `costClass` (enum: `"free"` | `"low"` | `"medium"` | `"high"`), `sideEffects` (boolean, default false). These fields are informational and SHALL NOT affect validation.

#### Scenario: Behavioral metadata present
- **WHEN** a contract declares `behavior: { latencyClass: "slow", sideEffects: true }`
- **THEN** validation SHALL pass and the metadata SHALL be preserved for registry/policy use

#### Scenario: Behavioral metadata omitted
- **WHEN** a contract omits the `behavior` field
- **THEN** validation SHALL pass with no defaults applied for behavioral metadata
