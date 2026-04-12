## ADDED Requirements

### Requirement: Cache root directory
The cache manager SHALL use `~/.skills/` as the root directory for all local state. If the directory does not exist, it SHALL be created on first use. The root location SHALL be overridable via the `SKILL_HOME` environment variable.

#### Scenario: Default cache location
- **WHEN** the cache manager initializes with no `SKILL_HOME` set
- **THEN** it SHALL use `~/.skills/` as the root directory, creating it if necessary

#### Scenario: Custom cache location
- **WHEN** the `SKILL_HOME` environment variable is set to `/opt/skills`
- **THEN** the cache manager SHALL use `/opt/skills/` as the root directory

### Requirement: Artifact storage layout
Installed skill artifacts SHALL be stored under `{root}/cache/artifacts/{identity-hash}/` where `{identity-hash}` is a SHA-256 hash of the skill name + version string. The artifact directory SHALL contain the complete skill package (manifest + content files) as copied from the source.

#### Scenario: Install stores artifact
- **WHEN** a skill `openspec-explore@1.0.0` is installed
- **THEN** the cache manager SHALL copy the skill package to `~/.skills/cache/artifacts/{sha256("openspec-explore@1.0.0")}/`

#### Scenario: Same skill, different version
- **WHEN** `openspec-explore@1.0.0` and `openspec-explore@1.1.0` are both installed
- **THEN** each SHALL have a separate directory under `cache/artifacts/` with a different identity hash

### Requirement: Metadata cache
The cache manager SHALL maintain a metadata index at `{root}/metadata/index.json` listing all installed skills with: `name`, `version`, `identityHash`, `installedAt` (ISO 8601 timestamp), `manifestPath` (path to cached manifest), and `source` (original install source — local path or URL).

#### Scenario: Metadata index after install
- **WHEN** a skill is installed
- **THEN** the metadata index SHALL contain an entry for the skill with all required fields

#### Scenario: Metadata index after uninstall
- **WHEN** a skill is removed from the cache
- **THEN** its entry SHALL be removed from the metadata index

#### Scenario: Metadata index is consistent
- **WHEN** the cache manager reads the metadata index
- **THEN** every entry SHALL reference an existing artifact directory, and orphaned entries SHALL be flagged on integrity check

### Requirement: Activation state per runtime
The cache manager SHALL maintain activation state files at `{root}/activations/{runtimeId}.json`. Each file SHALL contain an array of activation records with: `skillName`, `version`, `identityHash`, `activatedAt` (ISO 8601 timestamp), and `outputPaths` (array of file paths written during activation).

#### Scenario: Activate skill for Claude Code
- **WHEN** a skill is activated for `claude-code`
- **THEN** `~/.skills/activations/claude-code.json` SHALL contain an activation record for that skill

#### Scenario: Deactivate skill
- **WHEN** a skill is deactivated for a runtime
- **THEN** the activation record SHALL be removed from the runtime's activation file and the generated output files listed in `outputPaths` SHALL be deleted

#### Scenario: Activation state survives cache reads
- **WHEN** the cache manager reads activation state for `codex`
- **THEN** it SHALL return only skills activated for the `codex` runtime, not all installed skills

### Requirement: Install and activation separation
Installing a skill SHALL NOT automatically activate it for any runtime. Activation SHALL be a separate explicit operation. A skill MAY be installed but not activated for any runtime.

#### Scenario: Install without activation
- **WHEN** the user runs `skill install ./my-skill`
- **THEN** the skill SHALL appear in the metadata index but SHALL NOT appear in any activation state file

#### Scenario: Activate requires prior install
- **WHEN** the user attempts to activate a skill that is not installed
- **THEN** the cache manager SHALL return an error indicating the skill must be installed first

### Requirement: User configuration
The cache manager SHALL read user configuration from `{root}/config.json`. Configuration fields SHALL include: `defaultTargets` (array of runtime IDs to activate by default, optional), `policy` (permission allowlist/blocklist, optional). If the config file does not exist, all defaults SHALL apply.

#### Scenario: Default targets configured
- **WHEN** `config.json` contains `{ defaultTargets: ["claude-code"] }`
- **THEN** the `skill activate` command with no `--target` flag SHALL activate only for Claude Code

#### Scenario: No config file
- **WHEN** `~/.skills/config.json` does not exist
- **THEN** all enabled adapters from the manifest SHALL be used as targets

### Requirement: Cache integrity check
The cache manager SHALL provide an integrity check operation that: verifies every metadata index entry references an existing artifact directory, verifies every activation record references an installed skill, and reports orphaned artifacts or broken activations.

#### Scenario: Clean cache
- **WHEN** the integrity check runs and all entries are consistent
- **THEN** it SHALL report no issues

#### Scenario: Orphaned artifact
- **WHEN** an artifact directory exists in `cache/artifacts/` but has no corresponding metadata index entry
- **THEN** the integrity check SHALL report the orphaned artifact and offer to clean it up
