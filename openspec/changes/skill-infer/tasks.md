## Tasks

### Phase 1: Core Types & Crate Setup

- [x] Create `crates/aule-infer/` crate with `Cargo.toml` — dependencies: `aule-schema`, `serde`, `serde_json`, `serde_yaml`, `reqwest`, `glob`
- [x] Add `aule-infer` to workspace `Cargo.toml`
- [x] Define `DiscoveredSkill`, `SourceFormat` types
- [x] Define `InferredSignals`, `ExecutableInfo`, `SignalSource` types
- [x] Define `LlmAssessment`, `SuggestedSkill`, `SuggestedTool` types
- [x] Define `InferError` error enum covering all failure modes
- [x] Define `ScanResult` type with skills + warnings

### Phase 2: Stage 1 — Skill Scanners

- [x] Implement `ClaudeSkillScanner` — glob `.claude/skills/**/*.md`, parse YAML frontmatter
- [x] Implement `CodexSkillScanner` — glob `.codex/skills/**/*.md`, parse YAML frontmatter
- [x] Implement `ClaudeCommandScanner` — glob `.claude/commands/**/*.md`, extract command names
- [x] Implement `PluginScanner` — parse `plugin.json` at repo root
- [x] Implement `SkillMdScanner` — find standalone `SKILL.md` files
- [x] Implement `scan_all` orchestrator — run all scanners, merge, deduplicate
- [x] Write unit tests for each scanner with fixture directories
- [x] Write integration test for `scan_all` with mixed content repo

### Phase 3: Stage 2 — Signal Gatherers

- [x] Implement `GenericGatherer` — README detection, file tree, license, executables
- [x] Implement `NpmGatherer` — parse `package.json`
- [x] Implement `PythonGatherer` — parse `pyproject.toml` / `setup.py` / `setup.cfg`
- [x] Implement `RustGatherer` — parse `Cargo.toml`
- [x] Implement `GoGatherer` — parse `go.mod`, detect `cmd/`
- [x] Implement `gather_signals` orchestrator — run generic + language-specific, merge
- [x] Write unit tests for each gatherer with fixture files
- [x] Write integration test for `gather_signals` with real repo structures

### Phase 4: LLM Assessor

- [x] Implement Claude API client — `ANTHROPIC_API_KEY`, model selection, timeout
- [x] Design system prompt — task definition, ManifestV2 schema reference, examples
- [x] Implement request builder — serialize `InferredSignals` + README into prompt
- [x] Implement response parser — parse structured JSON into `LlmAssessment`
- [x] Implement retry logic — one retry on malformed response
- [x] Implement error handling — NoApiKey, LlmUnavailable, LlmRateLimit, LlmResponseParse
- [x] Write unit tests with mocked API responses
- [x] Write integration test against live API (behind feature flag or env var gate)

### Phase 5: Manifest Builder

- [x] Implement `build_from_discovered` — `Vec<DiscoveredSkill>` → `ManifestV2`
- [x] Implement `build_from_assessment` — `LlmAssessment` + `InferredSignals` → `ManifestV2`
- [x] Implement YAML serialization with field ordering
- [x] Implement post-build validation via `aule-schema` parser
- [x] Implement file path existence validation
- [x] Write unit tests for both builder paths
- [x] Write round-trip test: build → serialize → parse → compare

### Phase 6: CLI — `skill infer` Command

- [x] Add `infer` subcommand to CLI argument parser
- [x] Implement source resolution (local path vs git URL)
- [x] Implement existing `skill.yaml` check with `--force` override
- [x] Wire Stage 1 → Stage 2 pipeline with status output
- [x] Implement interactive confirmation for Stage 2 (Y/n/edit)
- [x] Implement `--install` flag — write manifest + hand off to install
- [x] Implement `--output` flag — write to specified path
- [x] Implement `--json` flag — structured JSON output
- [x] Implement `--yes` flag — auto-accept
- [x] Add `aule-infer` dependency to `aule-cli/Cargo.toml`
- [x] Write CLI integration tests

### Phase 7: CLI — `skill install --infer` Flag

- [x] Add `--infer` flag to install command argument parser
- [x] Implement fallback logic: no manifest → run inference pipeline
- [x] Implement manifest write-back to source directory before install
- [x] Handle git source cleanup (temp dir with injected skill.yaml)
- [x] Skip `--infer` for registry sources silently
- [x] Write CLI integration tests for `--infer` flag

### Phase 8: Testing & Validation

- [x] Create test fixture repos: skill-with-claude-skills, skill-with-plugin-json, npm-cli-tool, bare-readme-only, binary-assets-only, existing-skill-yaml
- [x] End-to-end test: `skill infer` on fixture with `.claude/skills/` → valid manifest
- [x] End-to-end test: `skill infer --install` on fixture → installed skill
- [x] End-to-end test: `skill install --infer` on fixture without manifest → inferred + installed
- [x] Verify `cargo test -p aule-infer` passes all unit + integration tests
- [x] Verify `cargo test` (full workspace) passes with new crate
