## NEW Requirements

### Requirement: `skill adapters list`
The CLI SHALL provide `skill adapters list` that displays all available adapters — built-in, user-installed, and (if run from within a skill package) skill-bundled. Each entry SHALL show: id, type (config/script/built-in), source, and description.

#### Scenario: Default listing
- **GIVEN** 3 built-in adapters and 1 user-installed adapter `gemini`
- **WHEN** running `skill adapters list`
- **THEN** output SHALL show all 4 adapters with their type and source

#### Scenario: JSON output
- **WHEN** running `skill adapters list --json`
- **THEN** output SHALL be a JSON array of adapter objects with `id`, `type`, `source`, and `description` fields

### Requirement: `skill adapters add`
The CLI SHALL provide `skill adapters add` to install an adapter from a local path or git URL into `~/.skills/adapters/<id>/`.

#### Scenario: Add from local path
- **GIVEN** a directory `./my-adapter/` containing a valid `adapter.yaml` with `id: gemini`
- **WHEN** running `skill adapters add --path ./my-adapter/`
- **THEN** the adapter SHALL be copied to `~/.skills/adapters/gemini/` and be immediately available

#### Scenario: Add from git URL
- **WHEN** running `skill adapters add --git https://github.com/user/gemini-adapter.git`
- **THEN** the repo SHALL be cloned, the `adapter.yaml` parsed for the id, and the contents placed in `~/.skills/adapters/<id>/`

#### Scenario: Adapter already installed
- **GIVEN** an adapter `gemini` already installed at `~/.skills/adapters/gemini/`
- **WHEN** running `skill adapters add --path ./updated-gemini/`
- **THEN** the CLI SHALL prompt for confirmation before overwriting (or accept `--force`)

#### Scenario: Invalid adapter directory
- **GIVEN** a directory without an `adapter.yaml`
- **WHEN** running `skill adapters add --path ./not-an-adapter/`
- **THEN** the CLI SHALL error: "No adapter.yaml found in ./not-an-adapter/"

### Requirement: `skill adapters remove`
The CLI SHALL provide `skill adapters remove <id>` to remove a user-installed adapter.

#### Scenario: Remove user-installed
- **GIVEN** a user-installed adapter `gemini`
- **WHEN** running `skill adapters remove gemini`
- **THEN** `~/.skills/adapters/gemini/` SHALL be deleted

#### Scenario: Cannot remove built-in
- **WHEN** running `skill adapters remove claude-code`
- **THEN** the CLI SHALL error: "Cannot remove built-in adapter 'claude-code'. You can override it by installing a custom version with `skill adapters add`."

### Requirement: `skill adapters info`
The CLI SHALL provide `skill adapters info <id>` that displays detailed information about an adapter: all fields from adapter.yaml, source (built-in/user-installed/skill-bundled), installation path, and protocol version.

#### Scenario: Info for built-in adapter
- **WHEN** running `skill adapters info claude-code`
- **THEN** output SHALL show the adapter's path templates, command support, extra fields, and source as "built-in"

#### Scenario: Info for unknown adapter
- **WHEN** running `skill adapters info nonexistent`
- **THEN** the CLI SHALL error: "Unknown adapter 'nonexistent'"

### Requirement: `skill adapters test`
The CLI SHALL provide `skill adapters test <id>` that tests an adapter's correctness by running it against a skill manifest.

#### Scenario: Test with default manifest
- **WHEN** running `skill adapters test gemini`
- **THEN** the CLI SHALL generate a minimal synthetic skill manifest, run validation (if the adapter has a validate script), run generation, and verify the output

#### Scenario: Test with user-provided skill
- **WHEN** running `skill adapters test gemini --path ./my-skill/`
- **THEN** the CLI SHALL use the provided skill package for testing instead of a synthetic manifest

#### Scenario: Test verifies output integrity
- **WHEN** testing a script-based adapter
- **THEN** the CLI SHALL verify: script is executable, output is valid JSON, file paths contain no traversal, content is valid UTF-8

#### Scenario: Test verifies config-based adapter
- **WHEN** testing a config-based adapter
- **THEN** the CLI SHALL verify: path templates contain required placeholders (`{name}` for skills), generated frontmatter is valid YAML

#### Scenario: Test reports results
- **THEN** the test command SHALL report pass/fail for each check, with details on failures
