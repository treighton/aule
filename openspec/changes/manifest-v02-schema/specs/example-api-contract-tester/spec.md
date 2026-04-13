## ADDED Requirements

### Requirement: Reference skill package exists
The repository SHALL include a complete example skill package at `examples/api-contract-tester/` demonstrating all v0.2.0 manifest capabilities: multi-skill, executable tools, lifecycle hooks, and wrapper script generation.

#### Scenario: Example directory structure
- **WHEN** a developer looks at `examples/api-contract-tester/`
- **THEN** they SHALL find:
  - `skill.yaml` — v0.2.0 manifest with skills, tools, hooks, and files
  - `content/contract-tester.md` — main skill prose with multi-phase agentic workflow
  - `content/spec-linter.md` — secondary skill for spec validation
  - `logic/tools/generate.ts` — test harness generator
  - `logic/tools/run-tests.ts` — test executor
  - `logic/tools/report.ts` — report generator
  - `logic/hooks/setup.sh` — onInstall hook (runs npm install)
  - `logic/hooks/verify-runtime.sh` — onActivate hook (checks Node.js version)
  - `logic/package.json` — Node.js dependencies for tool scripts

### Requirement: Manifest exercises all new fields
The example `skill.yaml` SHALL use `schemaVersion: "0.2.0"` and include all new top-level fields: `files`, `skills` (with at least two skills), `tools` (with at least two tools), and `hooks` (with at least `onInstall`).

#### Scenario: Manifest validates cleanly
- **WHEN** `skill validate --path examples/api-contract-tester/` is run
- **THEN** validation SHALL pass with zero errors

### Requirement: Skills demonstrate different interfaces
The package SHALL include at least two skills with meaningfully different interfaces — different permissions, different determinism levels, and different I/O schemas.

#### Scenario: Contract-tester vs spec-linter
- **WHEN** the two skills' interfaces are compared
- **THEN** `contract-tester` SHALL declare `determinism: "bounded"` with all four permission types and structured I/O
- **THEN** `spec-linter` SHALL declare `determinism: "deterministic"` with only `filesystem.read` and simpler I/O

### Requirement: Tools are functional executables
Each tool script in `logic/tools/` SHALL be a real, executable script (not a placeholder) that accepts JSON input as a positional argument and writes JSON output to stdout.

#### Scenario: Generate tool produces output
- **WHEN** the `generate` tool is invoked with a valid OpenAPI spec path
- **THEN** it SHALL parse the spec, generate test file stubs, and output JSON with `status`, `testCount`, and `files` fields

#### Scenario: Tool handles invalid input
- **WHEN** a tool is invoked with malformed JSON or missing required fields
- **THEN** it SHALL output a JSON error response with `status: "error"` and a descriptive `message`

### Requirement: Skill content demonstrates agentic loop
The `contract-tester` skill's markdown SHALL demonstrate a multi-phase workflow: deterministic code execution (generate, run tests) followed by an agentic diagnosis loop (read source, correlate failures, propose fixes, re-run).

#### Scenario: Three-phase workflow
- **WHEN** an agent reads `content/contract-tester.md`
- **THEN** it SHALL find Phase 1 (Generate — deterministic, uses `generate` tool), Phase 2 (Execute — deterministic, uses `run-tests` tool), and Phase 3 (Diagnose — agentic, reads source code, proposes fixes, loops on re-run)

### Requirement: Adapter output matches for example
The real skills test (`crates/aule-adapter/tests/real_skills_test.rs`) SHALL include the `api-contract-tester` example and verify that `skill build` output matches committed adapter files.

#### Scenario: Build output matches committed files
- **WHEN** `cargo test -p aule-adapter --test real_skills_test` is run
- **THEN** the generated output for `api-contract-tester` SHALL match the committed files in `.claude/skills/api-contract-tester/` including SKILL.md, wrapper scripts, and copied logic files
