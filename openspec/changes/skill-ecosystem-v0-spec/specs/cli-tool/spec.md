## ADDED Requirements

### Requirement: CLI binary name and invocation
The CLI SHALL be invoked as `skill` (or `npx @aule/cli`). All commands SHALL follow the pattern `skill <command> [options]`. The CLI SHALL display usage help when invoked with no arguments or with `--help`.

#### Scenario: No arguments shows help
- **WHEN** the user runs `skill` with no arguments
- **THEN** the CLI SHALL display a list of available commands with brief descriptions

#### Scenario: Help flag on command
- **WHEN** the user runs `skill build --help`
- **THEN** the CLI SHALL display detailed usage for the `build` command including all options

### Requirement: `skill init` command
`skill init` SHALL scaffold a new skill package in the current directory or a named subdirectory. It SHALL create: `skill.yaml` (manifest with placeholder values), `content/skill.md` (empty skill body with frontmatter hint), and `content/commands/` (empty directory). It SHALL accept an optional `--name` flag to set the skill name, defaulting to the directory name.

#### Scenario: Init in empty directory
- **WHEN** the user runs `skill init --name my-skill` in an empty directory
- **THEN** the CLI SHALL create `skill.yaml` with `name: "my-skill"` and placeholder fields, `content/skill.md` with a starter template, and `content/commands/`

#### Scenario: Init in non-empty directory
- **WHEN** the user runs `skill init` in a directory that already contains a `skill.yaml`
- **THEN** the CLI SHALL exit with an error indicating a skill package already exists

### Requirement: `skill validate` command
`skill validate` SHALL parse the manifest, validate it against the JSON Schema, validate the contract, check that all referenced content files exist, and report all errors and warnings. It SHALL exit with code 0 on success and code 1 on any error.

#### Scenario: Valid skill package
- **WHEN** the user runs `skill validate` in a directory with a valid `skill.yaml` and all referenced files
- **THEN** the CLI SHALL print a success message and exit with code 0

#### Scenario: Validation errors
- **WHEN** the user runs `skill validate` and the manifest has missing required fields
- **THEN** the CLI SHALL print each error with the field path and description, then exit with code 1

#### Scenario: Validation warnings
- **WHEN** the user runs `skill validate` and the manifest has unknown extension fields or unknown permission strings
- **THEN** the CLI SHALL print warnings but still exit with code 0

### Requirement: `skill build` command
`skill build` SHALL validate the manifest, then invoke the adapter generator for all enabled runtime targets. It SHALL accept an optional `--target` flag to build for a specific runtime only. It SHALL accept an optional `--output` flag to specify the output root directory (default: current working directory). It SHALL report which files were generated.

#### Scenario: Build for all targets
- **WHEN** the user runs `skill build` with adapters enabled for `claude-code` and `codex`
- **THEN** the CLI SHALL generate output for both runtimes and list all created files

#### Scenario: Build for single target
- **WHEN** the user runs `skill build --target claude-code`
- **THEN** the CLI SHALL generate output only for Claude Code

#### Scenario: Build fails on invalid manifest
- **WHEN** the user runs `skill build` and the manifest fails validation
- **THEN** the CLI SHALL print validation errors and exit with code 1 without generating any files

### Requirement: `skill install` command
`skill install` SHALL install a skill from a local path or (in future versions) a remote identity. In v0 it SHALL accept a local directory path containing a `skill.yaml`. It SHALL validate the manifest, copy the skill artifacts to the local cache (`~/.skills/cache/artifacts/{hash}/`), and record the installation in cache metadata.

#### Scenario: Install from local path
- **WHEN** the user runs `skill install ./my-skill`
- **THEN** the CLI SHALL validate the manifest at `./my-skill/skill.yaml`, copy the skill package to the cache, and print the installation location

#### Scenario: Install path has no manifest
- **WHEN** the user runs `skill install ./not-a-skill`
- **THEN** the CLI SHALL exit with an error indicating no `skill.yaml` found at the path

### Requirement: `skill activate` command
`skill activate` SHALL bind an installed skill to one or more runtime targets by generating adapter output in the appropriate runtime directories. It SHALL accept `--target` to specify which runtimes (default: all enabled adapters). It SHALL update the activation state in `~/.skills/activations/{runtime}.json`.

#### Scenario: Activate for all runtimes
- **WHEN** the user runs `skill activate openspec-explore`
- **THEN** the CLI SHALL generate adapter output for all enabled runtimes and update activation state for each

#### Scenario: Activate for specific runtime
- **WHEN** the user runs `skill activate openspec-explore --target claude-code`
- **THEN** the CLI SHALL generate adapter output only in `.claude/skills/` and update only `claude-code.json` activation state

#### Scenario: Activate skill not installed
- **WHEN** the user runs `skill activate nonexistent-skill`
- **THEN** the CLI SHALL exit with an error indicating the skill is not installed

### Requirement: `skill list` command
`skill list` SHALL display installed skills and their activation status. It SHALL accept `--installed` to show only installed skills and `--active` to show only activated skills. Default (no flag) SHALL show all installed skills with their activation status per runtime.

#### Scenario: List with mixed state
- **WHEN** the user has 3 installed skills, 2 activated for claude-code, 1 activated for codex
- **THEN** the CLI SHALL display a table showing each skill name, version, and activation status per runtime

#### Scenario: No skills installed
- **WHEN** the user has no skills installed
- **THEN** the CLI SHALL display a message indicating no skills are installed

### Requirement: Exit codes and error output
The CLI SHALL use exit code 0 for success, 1 for validation/user errors, and 2 for unexpected/internal errors. Error messages SHALL be written to stderr. Normal output SHALL be written to stdout. The CLI SHALL support `--json` flag on all commands to output machine-readable JSON instead of human-formatted text.

#### Scenario: JSON output mode
- **WHEN** the user runs `skill validate --json` and validation fails
- **THEN** the CLI SHALL write a JSON object to stdout with `{ success: false, errors: [...] }` and exit with code 1

#### Scenario: Human output mode (default)
- **WHEN** the user runs `skill validate` without `--json`
- **THEN** the CLI SHALL write human-readable formatted error messages to stderr
