## NEW Requirements

### Requirement: Three-source adapter discovery
The adapter registry SHALL discover adapters from three sources: user-installed (`~/.skills/adapters/<id>/adapter.yaml`), skill-bundled (`<package>/adapters/<id>/adapter.yaml`), and built-in (compiled into the binary).

#### Scenario: User-installed adapter found
- **GIVEN** a directory `~/.skills/adapters/gemini/` containing a valid `adapter.yaml`
- **THEN** `registry.by_id("gemini")` SHALL return the adapter definition

#### Scenario: Skill-bundled adapter found
- **GIVEN** a skill package at `./my-skill/` with `./my-skill/adapters/cursor/adapter.yaml`
- **WHEN** building that skill
- **THEN** the `cursor` adapter SHALL be available for that build

#### Scenario: Built-in adapter found
- **GIVEN** no external adapter with id `claude-code`
- **THEN** `registry.by_id("claude-code")` SHALL return the compiled-in Claude Code adapter definition

### Requirement: Precedence ordering
When the same adapter ID exists in multiple sources, the registry SHALL use this precedence: user-installed (highest) > skill-bundled > built-in (lowest).

#### Scenario: User overrides built-in
- **GIVEN** a user-installed adapter at `~/.skills/adapters/claude-code/adapter.yaml` AND the compiled-in claude-code adapter
- **THEN** `registry.by_id("claude-code")` SHALL return the user-installed version

#### Scenario: User overrides skill-bundled
- **GIVEN** a user-installed adapter `gemini` AND a skill-bundled adapter `gemini`
- **THEN** the user-installed version SHALL be used

### Requirement: List all available adapters
`registry.all()` SHALL return a deduplicated list of all available adapters across all sources, with precedence applied. Each entry SHALL indicate its source (user-installed, skill-bundled, built-in).

#### Scenario: Merged list with deduplication
- **GIVEN** built-in adapters `claude-code`, `codex`, `pi` AND user-installed adapter `gemini` AND skill-bundled adapter `codex`
- **THEN** `registry.all()` SHALL return 4 adapters: `claude-code` (built-in), `codex` (built-in, since built-in takes precedence over—wait, user > skill > built-in, so skill-bundled codex beats built-in), `pi` (built-in), `gemini` (user-installed)

#### Scenario: Correct — skill-bundled overrides built-in
- **GIVEN** built-in `codex` AND skill-bundled `codex` with different path templates
- **THEN** `registry.all()` SHALL return the skill-bundled version of `codex`

### Requirement: Unknown adapter ID handling
When `by_id()` finds no adapter across any source, it SHALL return `None`. The caller (build command) SHALL report a clear error listing available adapter IDs.

#### Scenario: Unknown adapter
- **WHEN** `skill build --target unknown-runtime` is run
- **THEN** the CLI SHALL report an error like "Unknown adapter 'unknown-runtime'. Available adapters: claude-code, codex, pi, gemini"

### Requirement: Built-in adapters as config definitions
The three built-in adapters (claude-code, codex, pi) SHALL be expressed internally as config-based `AdapterDef::Config` values, not as separate constructors with special-cased logic.

#### Scenario: Pi adapter uses config-based extra fields
- **GIVEN** the built-in Pi adapter with `extra_fields: ["allowed-tools", "disable-model-invocation"]`
- **WHEN** a skill manifest has `adapters.pi.allowed-tools: "Read,Write"`
- **THEN** the generated SKILL.md frontmatter SHALL include `allowed-tools: Read,Write` — handled by the generic config-based path, not `PI_EXTRA_FIELDS`
