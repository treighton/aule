## ADDED Requirements

### Requirement: Tool declaration in manifest
The manifest SHALL support a top-level `tools` map where each key is a tool name (kebab-case) and each value is a tool definition object.

#### Scenario: Valid tool declaration
- **WHEN** a manifest contains a `tools` map with a tool named `generate` that includes `description`, `using`, `entrypoint`, `input`, and `output` fields
- **THEN** the parser SHALL accept the manifest and produce a typed `Tool` struct for each entry

#### Scenario: Tool without required fields
- **WHEN** a manifest contains a tool entry missing `description`, `using`, or `entrypoint`
- **THEN** the parser SHALL reject the manifest with a validation error identifying the missing field

#### Scenario: Tool name validation
- **WHEN** a manifest contains a tool with a name that is not kebab-case (e.g., `generateTests` or `Generate_Tests`)
- **THEN** the parser SHALL reject the manifest with a validation error

### Requirement: Per-tool runtime declaration
Each tool SHALL declare its runtime via a `using` field with one of the supported values: `node`, `python`, `shell`.

#### Scenario: Valid runtime
- **WHEN** a tool declares `using: "node"`
- **THEN** the parser SHALL accept the declaration and record the runtime as Node.js

#### Scenario: Unknown runtime
- **WHEN** a tool declares `using: "ruby"` or another unsupported runtime
- **THEN** the parser SHALL emit a validation warning (not error) for forward compatibility

### Requirement: Optional runtime version constraint
Each tool MAY declare a `version` field containing a semver constraint string for the declared runtime.

#### Scenario: Version constraint present
- **WHEN** a tool declares `using: "node"` and `version: ">= 18"`
- **THEN** the parser SHALL store the version constraint and the adapter SHALL include it in generated tool documentation

#### Scenario: Version constraint absent
- **WHEN** a tool declares `using: "node"` with no `version` field
- **THEN** the parser SHALL accept the tool and assume any version of the runtime is acceptable

### Requirement: Tool entrypoint validation
Each tool's `entrypoint` field SHALL reference a file path relative to the manifest directory. The file MUST exist within the paths matched by the `files` include globs.

#### Scenario: Entrypoint exists
- **WHEN** a tool declares `entrypoint: "logic/tools/generate.ts"` and `files` includes `"logic/**"`
- **THEN** validation SHALL pass

#### Scenario: Entrypoint not found
- **WHEN** a tool declares `entrypoint: "logic/tools/missing.ts"` and the file does not exist on disk
- **THEN** validation SHALL fail with an error identifying the missing entrypoint

### Requirement: Typed tool input/output
Each tool SHALL declare `input` and `output` fields as JSON Schema objects defining the tool's expected input and output shapes.

#### Scenario: Structured input and output
- **WHEN** a tool declares `input: { type: "object", properties: { spec: { type: "string" } }, required: ["spec"] }`
- **THEN** the parser SHALL validate the JSON Schema and store it for documentation generation

#### Scenario: Invalid JSON Schema in tool definition
- **WHEN** a tool's `input` contains an invalid JSON Schema (e.g., `type: "nonexistent"`)
- **THEN** the parser SHALL reject the manifest with a validation error

### Requirement: Tools are package-scoped
Tools declared at the top level SHALL be available to all skills within the package. Skills reference tools by name in their prose content; no formal binding between skills and tools is required in v0.2.0.

#### Scenario: Multiple skills using the same tool
- **WHEN** a package declares two skills and one tool, and both skill entrypoints reference the tool by name in their markdown
- **THEN** the adapter SHALL generate wrapper scripts accessible from both skills' generated directories
