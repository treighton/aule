## ADDED Requirements

### Requirement: Resolver input model
The resolver SHALL accept a resolution request containing: `skillName` (string, required), `versionConstraint` (semver range string, optional — default `"*"`), `runtimeTarget` (string, optional — filters to specific runtime), `environment` (object describing OS, architecture, available runtimes — optional in v0), and `policy` (object with trust/permission policy overrides — optional in v0).

#### Scenario: Minimal resolution request
- **WHEN** the resolver receives `{ skillName: "openspec-explore" }`
- **THEN** it SHALL resolve the latest available version for all enabled runtimes

#### Scenario: Version-constrained request
- **WHEN** the resolver receives `{ skillName: "openspec-explore", versionConstraint: "^1.0.0" }`
- **THEN** it SHALL resolve the highest version matching the constraint

#### Scenario: Runtime-filtered request
- **WHEN** the resolver receives `{ skillName: "openspec-explore", runtimeTarget: "claude-code" }`
- **THEN** it SHALL resolve only the Claude Code adapter and skip Codex

### Requirement: Resolution output model (install plan)
The resolver SHALL return an install plan containing: `skillName` (string), `resolvedVersion` (semver string), `contractVersion` (semver string), `adapters` (array of resolved adapter descriptors per runtime), `artifact` (object describing the source/artifact to fetch — path, URL, or cache reference), `permissions` (array of permission strings from the contract), and `riskTier` (computed maximum risk tier).

#### Scenario: Successful resolution
- **WHEN** the resolver finds a matching version with compatible adapters
- **THEN** it SHALL return a complete install plan with all fields populated

#### Scenario: No matching version
- **WHEN** no version satisfies the constraint
- **THEN** the resolver SHALL return an error with code `NO_MATCHING_VERSION`

#### Scenario: No compatible adapter for requested runtime
- **WHEN** a version exists but has no adapter for the requested runtime target
- **THEN** the resolver SHALL return an error with code `NO_COMPATIBLE_ADAPTER`

### Requirement: v0 local resolution strategy
In v0, the resolver SHALL support two resolution sources: (1) local filesystem path (a directory containing `skill.yaml`), and (2) the local cache (`~/.skills/cache/`). Remote resolution via metadata endpoints SHALL be specified but not required in v0.

#### Scenario: Resolve from local path
- **WHEN** the resolver is given a local path and that path contains a valid `skill.yaml`
- **THEN** it SHALL read the manifest, validate it, and produce an install plan referencing the local path as the artifact source

#### Scenario: Resolve from cache
- **WHEN** the resolver is given a skill name and the skill exists in the local cache
- **THEN** it SHALL read the cached manifest and produce an install plan referencing the cached artifact

#### Scenario: Skill not found in any source
- **WHEN** the skill name is not found locally or in the cache, and remote resolution is not available
- **THEN** the resolver SHALL return an error with code `SKILL_NOT_FOUND`

### Requirement: Adapter compatibility check
The resolver SHALL verify that each adapter in the resolved install plan is compatible with the target runtime. In v0, compatibility is determined by the adapter target ID matching a known runtime target in the adapter generator's target registry.

#### Scenario: Compatible adapter found
- **WHEN** a manifest declares `adapters: { claude-code: { enabled: true } }` and the resolution request targets `claude-code`
- **THEN** the resolver SHALL include the Claude Code adapter in the install plan

#### Scenario: Adapter disabled for target
- **WHEN** a manifest declares `adapters: { codex: { enabled: false } }` and the resolution request targets `codex`
- **THEN** the resolver SHALL exclude Codex from the install plan and return an error if no other targets were requested

### Requirement: Policy evaluation hook
The resolver SHALL apply policy checks after version and adapter resolution but before returning the install plan. In v0, policy evaluation SHALL check the resolved permissions against a local allowlist/blocklist if configured in `~/.skills/config.json`. If no policy is configured, all permissions SHALL be allowed.

#### Scenario: Permission blocked by policy
- **WHEN** the resolved skill requires `process.spawn` and the local policy blocklists `process.spawn`
- **THEN** the resolver SHALL return an error with code `PERMISSION_BLOCKED` and identify the blocked permission

#### Scenario: No policy configured
- **WHEN** no `~/.skills/config.json` exists or it contains no permission policy
- **THEN** the resolver SHALL allow all permissions and proceed with the install plan
