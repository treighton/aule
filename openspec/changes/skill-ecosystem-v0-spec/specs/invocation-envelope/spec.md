## ADDED Requirements

### Requirement: Request envelope format
The invocation request envelope SHALL be a JSON object containing: `skillName` (string), `contractVersion` (semver string), `input` (the skill input — string for prompt-based skills, object for structured-input skills), and `context` (optional object for additional invocation context such as working directory, runtime identity, or granted context).

#### Scenario: Prompt-based invocation
- **WHEN** a runtime invokes a prompt-based skill with text input
- **THEN** the request envelope SHALL be `{ skillName: "openspec-explore", contractVersion: "1.0.0", input: "investigate the auth system", context: { workingDir: "/home/user/project" } }`

#### Scenario: Structured invocation
- **WHEN** a runtime invokes a skill with structured input matching its contract input schema
- **THEN** the request envelope `input` field SHALL contain the structured object and the adapter SHALL validate it against the contract's input JSON Schema before forwarding

#### Scenario: Missing required envelope fields
- **WHEN** an invocation request omits `skillName` or `contractVersion`
- **THEN** the adapter SHALL reject the invocation with a standard envelope error before reaching the implementation

### Requirement: Response envelope format
The invocation response envelope SHALL be a JSON object containing: `status` (enum: `"success"` | `"error"`), `output` (the skill output — string for prompt-based skills, object for structured-output skills, present when status is `"success"`), and `metadata` (optional object for execution metadata such as duration, token usage, or telemetry hints).

#### Scenario: Successful prompt response
- **WHEN** a prompt-based skill completes successfully
- **THEN** the response envelope SHALL be `{ status: "success", output: "Here's what I found about the auth system...", metadata: { durationMs: 1200 } }`

#### Scenario: Structured output response
- **WHEN** a structured-output skill completes
- **THEN** the response `output` SHALL conform to the contract's output JSON Schema

### Requirement: Error envelope format
When `status` is `"error"`, the response SHALL contain an `error` object with: `code` (string — a standard code or a contract-defined custom code), `message` (human-readable string), and `details` (optional object with additional error context). Standard error codes SHALL include: `VALIDATION_ERROR`, `PERMISSION_DENIED`, `EXECUTION_ERROR`, `TIMEOUT`, `CONTRACT_MISMATCH`.

#### Scenario: Validation error
- **WHEN** invocation input fails contract input schema validation
- **THEN** the error envelope SHALL be `{ status: "error", error: { code: "VALIDATION_ERROR", message: "Input field 'query' is required", details: { field: "query" } } }`

#### Scenario: Custom error code from contract
- **WHEN** a skill returns an error with a code defined in its contract's `errors` array
- **THEN** the error envelope SHALL use the custom code (e.g., `CONTEXT_TOO_LARGE`) and it SHALL be recognizable by consumers who read the contract

### Requirement: Envelope versioning
The request and response envelopes SHALL include an `envelopeVersion` field (string, `"0.1.0"` for v0). Adapters receiving an envelope with an unsupported version SHALL return an error with code `ENVELOPE_VERSION_UNSUPPORTED`.

#### Scenario: Matching envelope version
- **WHEN** a request includes `envelopeVersion: "0.1.0"` and the adapter supports v0.1
- **THEN** the adapter SHALL process the request normally

#### Scenario: Unsupported envelope version
- **WHEN** a request includes `envelopeVersion: "2.0.0"` and the adapter only supports v0.1
- **THEN** the adapter SHALL return `{ status: "error", error: { code: "ENVELOPE_VERSION_UNSUPPORTED", message: "Adapter supports envelope version 0.1.0, received 2.0.0" } }`

### Requirement: Prompt-based skill passthrough
For v0 prompt-based skills (contract inputs/outputs are `"prompt"`), the invocation envelope is primarily a structural boundary for future use. The adapter for prompt-based skills in coding agents SHALL extract the `input` string and pass it as the prompt content to the skill's markdown body. The skill's textual response SHALL be wrapped in the response envelope `output` field.

#### Scenario: Prompt passthrough in Claude Code
- **WHEN** a Claude Code adapter invokes a prompt-based skill
- **THEN** the adapter SHALL extract `input` from the envelope, provide it along with the SKILL.md body to the runtime, and wrap the runtime's response in a response envelope

#### Scenario: Prompt passthrough is transparent to the skill author
- **WHEN** a skill author writes a prompt-based SKILL.md
- **THEN** the skill content SHALL not reference or depend on envelope structure — the adapter handles envelope wrapping/unwrapping
