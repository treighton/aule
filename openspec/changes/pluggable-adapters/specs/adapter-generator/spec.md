## MODIFIED Requirements

### Requirement: Generation dispatches by adapter type
The generation pipeline SHALL dispatch based on adapter type: `AdapterDef::Config` uses the built-in generation logic (frontmatter + content concatenation), `AdapterDef::Script` invokes the external generate script via subprocess.

#### Scenario: Config-based adapter generation
- **WHEN** `skill build --target gemini` where `gemini` is a config-based adapter
- **THEN** the built-in pipeline SHALL generate SKILL.md using the adapter's path templates and frontmatter config

#### Scenario: Script-based adapter generation
- **WHEN** `skill build --target cursor` where `cursor` is a script-based adapter
- **THEN** the CLI SHALL serialize manifest+content as JSON, invoke the generate script, parse the output, and write the returned files

### Requirement: Config-based generation uses adapter definition
The built-in generation pipeline SHALL read path templates, command configuration, and extra frontmatter fields from the `AdapterDef::Config` rather than from hardcoded constructors or constants.

#### Scenario: Custom skill path template
- **GIVEN** a config-based adapter with `paths.skill: ".runtime/{name}/SKILL.md"`
- **THEN** the generated skill file SHALL be written to `.runtime/contract-tester/SKILL.md`

#### Scenario: Custom extra fields
- **GIVEN** a config-based adapter with `frontmatter.extra_fields: ["model-preference"]` and a manifest with `adapters.gemini.model-preference: "gemini-pro"`
- **THEN** the generated frontmatter SHALL include `model-preference: gemini-pro`

### Requirement: Eliminate PI_EXTRA_FIELDS
The hardcoded `PI_EXTRA_FIELDS` constant SHALL be removed. The Pi adapter's extra fields SHALL be declared in its config-based adapter definition and processed by the generic config-based pipeline.

#### Scenario: Pi adapter extra fields via config
- **GIVEN** the built-in Pi adapter with config `extra_fields: ["allowed-tools", "disable-model-invocation"]`
- **WHEN** generating a skill with `adapters.pi.allowed-tools: "Read,Write"`
- **THEN** the frontmatter SHALL include `allowed-tools: Read,Write` — same behavior as today, different code path

### Requirement: Target resolution uses registry
`resolve_targets()` and `resolve_targets_v2()` SHALL use the adapter registry for ID lookup instead of `RuntimeTarget::by_id()`. Unknown IDs SHALL produce clear errors listing available adapters.

#### Scenario: Custom adapter in manifest
- **GIVEN** a manifest with `adapters: { gemini: { enabled: true } }` and a user-installed `gemini` adapter
- **WHEN** running `skill build`
- **THEN** the build SHALL generate output for the `gemini` adapter

#### Scenario: Unknown adapter in manifest
- **GIVEN** a manifest with `adapters: { unknown: { enabled: true } }` and no adapter with id `unknown`
- **WHEN** running `skill build`
- **THEN** the CLI SHALL warn about the unknown adapter and skip it (not fail the entire build)

### Requirement: Backward compatibility
All existing skills and adapter configurations SHALL produce byte-identical output after this change. The refactoring is internal — no user-facing behavior changes for built-in adapters.

#### Scenario: Existing real_skills_test passes
- **WHEN** running the `real_skills_test` integration test
- **THEN** all generated output SHALL match the committed adapter files exactly
