## NEW Requirements

### Requirement: Validation script protocol
A validation script SHALL receive the same JSON input on stdin as the generate script and SHALL output a JSON object to stdout with fields: `valid` (boolean), `errors` (array of objects with `field` and `message`), `warnings` (array of objects with `field` and `message`).

#### Scenario: Validation passes
- **GIVEN** a validate script that outputs `{ "valid": true, "errors": [], "warnings": [] }`
- **THEN** generation SHALL proceed normally

#### Scenario: Validation passes with warnings
- **GIVEN** a validate script that outputs `{ "valid": true, "errors": [], "warnings": [{ "field": "tools.lint", "message": "Shell tools may not work" }] }`
- **THEN** the CLI SHALL display warnings and proceed with generation

#### Scenario: Validation fails
- **GIVEN** a validate script that outputs `{ "valid": false, "errors": [{ "field": "skills.x.commands", "message": "Commands not supported" }] }`
- **THEN** the CLI SHALL display errors and skip generation for this adapter

### Requirement: Validation runs before generation
When an adapter defines a `validate` script, the CLI SHALL run validation before invoking generation. If validation fails (any errors), generation SHALL be skipped for that adapter. Other adapters in the same build SHALL still run.

#### Scenario: One adapter fails validation, others succeed
- **GIVEN** a build targeting adapters `claude-code` and `cursor`, where `cursor` validation fails
- **THEN** `claude-code` SHALL generate normally, `cursor` SHALL be skipped with an error message

### Requirement: Validation is optional
Adapters that omit the `validate` field SHALL skip validation entirely and proceed directly to generation.

#### Scenario: No validate script
- **GIVEN** an adapter.yaml without a `validate` field
- **THEN** `skill build` SHALL proceed to generation without a validation step

### Requirement: Validation script failure handling
If the validation script itself crashes (non-zero exit, invalid JSON output), the CLI SHALL treat it as a validation failure and skip generation for that adapter.

#### Scenario: Validation script crashes
- **GIVEN** a validate script that exits with code 1 and no JSON output
- **THEN** the CLI SHALL report the validation failure and skip generation
