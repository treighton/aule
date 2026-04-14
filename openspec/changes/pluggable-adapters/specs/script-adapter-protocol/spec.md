## NEW Requirements

### Requirement: Input JSON schema (v1)
The CLI SHALL pass a JSON object on stdin to the generate script containing: `protocol_version` (integer), `manifest` (full parsed manifest), `content` (resolved skill content, command content, and file contents), `adapter_config` (the AdapterConfig for this adapter from the manifest), and `options` (output_dir, base_path).

#### Scenario: v0.1.0 skill input
- **GIVEN** a v0.1.0 skill with one skill.md and two commands
- **THEN** stdin SHALL contain `content.skills.<name>` with the skill body, `content.commands.<name>` with command bodies keyed by command name

#### Scenario: v0.2.0 skill input with files
- **GIVEN** a v0.2.0 skill with `files: ["logic/**"]` matching 3 files
- **THEN** stdin SHALL contain `content.files` with all 3 file contents keyed by relative path

#### Scenario: Content is fully resolved
- **THEN** all file contents SHALL be read and included as strings in the JSON, so the script does not need filesystem access to the skill package

### Requirement: Output JSON schema
The generate script SHALL write a JSON object to stdout containing `files`: an array of objects with `relative_path` (string) and `content` (string). The CLI SHALL write each file to the resolved output directory.

#### Scenario: Script generates one file
- **GIVEN** a script that outputs `{ "files": [{ "relative_path": ".cursor/rules/my-skill.mdc", "content": "..." }] }`
- **THEN** the CLI SHALL write that file at the correct output path

#### Scenario: Script generates multiple files
- **GIVEN** a script that outputs 3 files
- **THEN** all 3 files SHALL be written, with parent directories created as needed

### Requirement: Error handling
When the generate script exits with a non-zero exit code, the CLI SHALL treat it as a generation failure. If stderr contains valid JSON with an `error` field, the CLI SHALL display that message. Otherwise, the CLI SHALL display the raw stderr output.

#### Scenario: Script fails with structured error
- **GIVEN** a script that exits 1 with stderr `{ "error": "unsupported manifest version" }`
- **THEN** the CLI SHALL display "Adapter 'cursor' failed: unsupported manifest version"

#### Scenario: Script fails with unstructured error
- **GIVEN** a script that exits 1 with stderr "python: No module named 'yaml'"
- **THEN** the CLI SHALL display the raw stderr as the error message

### Requirement: Output validation
The CLI SHALL validate script output before writing files: all paths must be valid UTF-8, must not contain path traversal (`..`), and must be relative (not absolute). Files exceeding a reasonable size limit (10MB) SHALL be rejected.

#### Scenario: Path traversal rejected
- **GIVEN** a script that outputs `{ "files": [{ "relative_path": "../../etc/passwd", "content": "..." }] }`
- **THEN** the CLI SHALL reject the output with an error about path traversal

#### Scenario: Absolute path rejected
- **GIVEN** a script that outputs an absolute path like `/usr/local/bin/evil`
- **THEN** the CLI SHALL reject the output

### Requirement: Protocol version negotiation
The CLI SHALL check the adapter's declared `protocol` against its maximum supported protocol version. If the adapter's protocol version is higher than what the CLI supports, the CLI SHALL error with a message suggesting the user upgrade Aulë.

#### Scenario: Unsupported protocol version
- **GIVEN** an adapter with `protocol: 2` and a CLI that supports up to protocol 1
- **THEN** the CLI SHALL error: "Adapter 'cursor' requires protocol v2 but this version of Aulë only supports up to v1. Please upgrade."

#### Scenario: Missing protocol field
- **GIVEN** an adapter.yaml without a `protocol` field
- **THEN** the CLI SHALL assume protocol version 1

### Requirement: Script execution environment
The generate and validate scripts SHALL be executed with the adapter directory as the working directory, so relative paths in `generate` and `validate` resolve correctly. The script SHALL inherit the user's environment variables.

#### Scenario: Working directory
- **GIVEN** an adapter installed at `~/.skills/adapters/cursor/` with `generate: ./generate.py`
- **THEN** the script SHALL be executed with cwd set to `~/.skills/adapters/cursor/`
