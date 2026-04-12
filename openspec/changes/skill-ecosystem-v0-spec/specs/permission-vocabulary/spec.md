## ADDED Requirements

### Requirement: Permission string format
Permissions SHALL be expressed as dot-separated hierarchical strings following the pattern `{category}.{scope}` or `{category}.{scope}.{qualifier}`. All permission strings SHALL be lowercase alphanumeric with dots as separators.

#### Scenario: Valid permission string
- **WHEN** a contract declares permission `"filesystem.read"`
- **THEN** validation SHALL accept the permission as well-formed

#### Scenario: Invalid permission format
- **WHEN** a contract declares permission `"FileSystem Read"` (spaces, mixed case)
- **THEN** validation SHALL reject the string as malformed

### Requirement: v0 permission vocabulary categories
The v0 vocabulary SHALL define the following permission categories and scopes:

**Filesystem:** `filesystem.read` (read files), `filesystem.write` (write/create files), `filesystem.write.workspace` (write only within current workspace)

**Network:** `network.external` (make outbound HTTP/HTTPS requests), `network.external.specific` (with URL pattern qualifier in extensions)

**Process:** `process.spawn` (spawn child processes), `process.spawn.specific` (named tool only, tool name in extensions)

**Runtime:** `runtime.context` (access runtime/session context beyond input)

#### Scenario: Filesystem read permission
- **WHEN** a skill needs to read codebase files to function
- **THEN** its contract SHALL declare `permissions: ["filesystem.read"]`

#### Scenario: Skill requiring external CLI tool
- **WHEN** a skill depends on spawning the `openspec` CLI
- **THEN** its contract SHALL declare `permissions: ["process.spawn"]` and MAY specify the tool name via `process.spawn.specific` with extensions

#### Scenario: Zero-permission skill
- **WHEN** a skill operates entirely on its prompt input with no file/network/process access
- **THEN** its contract SHALL declare `permissions: []`

### Requirement: Permission hierarchy implies narrower grants
A permission at a broader scope SHALL imply all narrower scopes within the same category. `filesystem.write` SHALL imply `filesystem.write.workspace`. Policy engines MAY use this hierarchy to grant broader or narrower access.

#### Scenario: Broad permission implies narrow
- **WHEN** a policy grants `filesystem.write`
- **THEN** a skill requiring `filesystem.write.workspace` SHALL be considered permitted

#### Scenario: Narrow grant does not imply broad
- **WHEN** a policy grants only `filesystem.write.workspace`
- **THEN** a skill requiring `filesystem.write` (unrestricted) SHALL NOT be considered permitted

### Requirement: Vocabulary extensibility
The permission vocabulary SHALL be extensible. Permissions not in the v0 vocabulary SHALL be accepted with a warning during validation but SHALL NOT cause validation failure. This ensures forward-compatibility as the vocabulary grows.

#### Scenario: Future permission accepted with warning
- **WHEN** a contract declares `permissions: ["gpu.compute"]` which is not in the v0 vocabulary
- **THEN** validation SHALL emit a warning noting the unknown permission and SHALL pass without error

### Requirement: Permission-to-trust mapping
Each v0 permission SHALL have an associated risk tier: `low` (filesystem.read, runtime.context), `medium` (filesystem.write.workspace, network.external), `high` (filesystem.write, process.spawn). The risk tier SHALL be available to policy engines for automated trust decisions.

#### Scenario: Risk tier lookup
- **WHEN** a policy engine evaluates a skill declaring `permissions: ["filesystem.write", "network.external"]`
- **THEN** the engine SHALL determine the maximum risk tier is `high` (from `filesystem.write`)

#### Scenario: Zero-permission risk
- **WHEN** a skill declares no permissions
- **THEN** the risk tier SHALL be `none`
