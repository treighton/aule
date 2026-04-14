## NEW Requirements

### Requirement: adapter.yaml schema for config-based adapters
An adapter definition file (`adapter.yaml`) SHALL declare an adapter's identity and configuration using the following required fields: `id` (string, kebab-case), `type` ("config"), `protocol` (integer, default 1), `description` (string). Optional fields: `author` (string), `paths`, `frontmatter`, `validate` (script path).

#### Scenario: Minimal config adapter
- **GIVEN** an `adapter.yaml` with `id: gemini`, `type: config`, `paths.skill: ".gemini/skills/{name}/SKILL.md"`
- **THEN** the adapter SHALL be loadable and usable for skill generation with default values for all optional fields

#### Scenario: Config adapter with command support
- **GIVEN** an `adapter.yaml` with `paths.commands.path: ".gemini/commands/{namespace}/{command_name}.md"`
- **THEN** the adapter SHALL support command generation using the provided path template and associated display_name, category, and tags templates

#### Scenario: Config adapter with extra frontmatter fields
- **GIVEN** an `adapter.yaml` with `frontmatter.extra_fields: ["allowed-tools", "model-preference"]`
- **THEN** the adapter SHALL include values for those fields from `AdapterConfig.extra` in the generated SKILL.md frontmatter

### Requirement: adapter.yaml schema for script-based adapters
A script-based adapter definition SHALL declare `id`, `type: script`, `protocol`, `description`, and `generate` (path to the generate script, relative to the adapter directory). Optional: `author`, `validate` (path to validation script).

#### Scenario: Minimal script adapter
- **GIVEN** an `adapter.yaml` with `id: cursor`, `type: script`, `generate: ./generate.py`
- **THEN** the adapter SHALL be loadable and SHALL invoke the generate script during `skill build`

#### Scenario: Script adapter with validation
- **GIVEN** an `adapter.yaml` with `validate: ./validate.py` and `generate: ./generate.py`
- **THEN** the adapter SHALL run validation before generation and skip generation if validation reports errors

### Requirement: adapter.yaml validation
The parser SHALL reject adapter definitions with missing required fields, unknown `type` values, or invalid `protocol` versions (non-integer, negative).

#### Scenario: Missing required field
- **GIVEN** an `adapter.yaml` missing the `id` field
- **THEN** parsing SHALL fail with a descriptive error indicating the missing field

#### Scenario: Unknown type
- **GIVEN** an `adapter.yaml` with `type: wasm`
- **THEN** parsing SHALL fail with an error listing valid types ("config", "script")

### Requirement: Path template placeholders
Config-based adapters SHALL support `{name}` in `paths.skill`, and `{namespace}` and `{command_name}` in `paths.commands.path`. Command display_name and tags templates SHALL support `{skill}` and `{command}` placeholders.

#### Scenario: Placeholder resolution
- **GIVEN** `paths.skill: ".runtime/skills/{name}/SKILL.md"` and skill name `contract-tester`
- **THEN** the resolved path SHALL be `.runtime/skills/contract-tester/SKILL.md`
