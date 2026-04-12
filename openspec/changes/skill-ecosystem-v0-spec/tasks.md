## 1. Cargo Workspace Setup

- [x] 1.1 Initialize Cargo workspace with root `Cargo.toml` and `crates/` directory
- [x] 1.2 Create crate stubs: `aule-schema`, `aule-resolver`, `aule-adapter`, `aule-cache`, `aule-cli` (binary)
- [x] 1.3 Configure shared dependencies in workspace `Cargo.toml`: `serde`, `serde_yaml`, `serde_json`, `thiserror`, `sha2`
- [x] 1.4 Add `clap` (derive) to `aule-cli`, `jsonschema` to `aule-schema`, `dirs` to `aule-cache`
- [x] 1.5 Verify `cargo build` and `cargo test` run across all crates

## 2. Schema Crate — Manifest

- [x] 2.1 Define `Manifest` struct with serde derives covering all required/optional fields from manifest-schema spec
- [x] 2.2 Author the JSON Schema file (`manifest.schema.json`) as an embedded asset or shipped file
- [x] 2.3 Implement `parse_manifest(yaml: &str) -> Result<Manifest, ManifestError>` using `serde_yaml`
- [x] 2.4 Implement `validate_manifest(manifest: &Manifest) -> ValidationResult` — checks JSON Schema compliance, returns errors and warnings
- [x] 2.5 Implement content path validation — checks all `content.skill` and `content.commands` paths exist on disk relative to a base path
- [x] 2.6 Write tests for: valid manifest, missing fields, bad name format, missing content files, extension namespaces, tag limits

## 3. Schema Crate — Contract

- [x] 3.1 Define `Contract` struct and `InputType` enum (`Prompt` | `Schema(serde_json::Value)`) with serde derives
- [x] 3.2 Author the contract JSON Schema file (`contract.schema.json`)
- [x] 3.3 Implement `parse_contract(source: ContractSource) -> Result<Contract, ContractError>` handling both inline objects and external file references
- [x] 3.4 Implement `validate_contract(contract: &Contract) -> ValidationResult` — checks schema, validates permission strings against vocabulary, applies defaults
- [x] 3.5 Write tests for: prompt-based contract, structured I/O contract, unknown permissions (warning), missing version, behavioral metadata

## 4. Schema Crate — Permission Vocabulary

- [x] 4.1 Define the v0 permission vocabulary as a const array of `PermissionDef` structs with category, scope, and risk tier
- [x] 4.2 Implement `validate_permission(perm: &str) -> PermissionCheck` — checks format and warns on unknown permissions
- [x] 4.3 Implement `max_risk_tier(permissions: &[String]) -> RiskTier` — returns computed risk tier enum
- [x] 4.4 Implement `implies_permission(granted: &str, required: &str) -> bool` — checks hierarchical implication
- [x] 4.5 Write tests for: valid permissions, hierarchy implication, unknown permission warning, risk tier computation

## 5. Schema Crate — Invocation Envelope

- [x] 5.1 Define `RequestEnvelope`, `ResponseEnvelope`, `ErrorEnvelope` structs with serde derives
- [x] 5.2 Implement `validate_request(envelope: &RequestEnvelope) -> Result<(), EnvelopeError>` — checks required fields and version
- [x] 5.3 Implement `validate_response(envelope: &ResponseEnvelope) -> Result<(), EnvelopeError>` — checks status/output/error presence
- [x] 5.4 Implement `ErrorEnvelope::new(code: StandardError, message: &str) -> ErrorEnvelope` helper
- [x] 5.5 Write tests for: valid request/response, missing fields, version mismatch, standard error codes, custom error codes

## 6. Schema Crate — Metadata Endpoint

- [x] 6.1 Define `MetadataDocument` and `VersionDescriptor` structs with serde derives
- [x] 6.2 Implement `validate_metadata_document(doc: &MetadataDocument) -> ValidationResult`
- [x] 6.3 Write tests for: complete document, missing fields, checksum format, multiple versions

## 7. Adapter Generator Crate

- [x] 7.1 Define `RuntimeTarget` struct and implement `claude_code()` and `codex()` target constructors with directory layouts and frontmatter schemas
- [x] 7.2 Implement `generate_skill_file(manifest: &Manifest, target: &RuntimeTarget, content: &str) -> GeneratedFile` — produces SKILL.md with mapped frontmatter + verbatim body
- [x] 7.3 Implement `generate_command_files(manifest: &Manifest, target: &RuntimeTarget, commands: &HashMap<String, String>) -> Vec<GeneratedFile>` — produces command files for targets that support commands
- [x] 7.4 Implement `generate(manifest: &Manifest, base_path: &Path, options: &GenerateOptions) -> Result<Vec<GeneratedFile>, GenerateError>` — orchestrates: validate → generate skills → generate commands → write `.generated` markers
- [x] 7.5 Implement output directory creation and file writing with overwrite-but-don't-delete semantics
- [x] 7.6 Write tests for: Claude Code output structure, Codex skips commands, frontmatter field mapping, body passthrough is byte-identical

## 8. Adapter Generator — Validation Against Real Skills

- [x] 8.1 Create a `skill.yaml` manifest for the `openspec-explore` skill sourced from the existing `.claude/skills/openspec-explore/SKILL.md`
- [x] 8.2 Run the adapter generator and diff output against existing `.claude/skills/openspec-explore/SKILL.md` and `.codex/skills/openspec-explore/SKILL.md`
- [x] 8.3 Fix any discrepancies until generated output matches existing hand-written adapter files
- [x] 8.4 Repeat for `openspec-propose`, `openspec-apply-change`, and `openspec-archive-change`
- [x] 8.5 Write integration test that generates all 4 OpenSpec skills and asserts output matches expected files (using `assert_eq!` on file contents)

## 9. Resolver Crate

- [x] 9.1 Define `ResolveRequest`, `InstallPlan`, and `ResolveError` types
- [x] 9.2 Implement `resolve_from_path(path: &Path, request: &ResolveRequest) -> Result<InstallPlan, ResolveError>` — local filesystem resolution
- [x] 9.3 Implement `resolve_from_cache(request: &ResolveRequest, cache: &CacheManager) -> Result<InstallPlan, ResolveError>` — resolution from local cache
- [x] 9.4 Implement `resolve(request: &ResolveRequest, sources: &ResolveSources) -> Result<InstallPlan, ResolveError>` — orchestrates cache → local → error
- [x] 9.5 Implement adapter compatibility check within resolution
- [x] 9.6 Implement policy evaluation — check permissions against config blocklist/allowlist
- [x] 9.7 Write tests for: successful resolution, version constraint filtering, no matching version, permission blocked, no compatible adapter

## 10. Cache Manager Crate

- [x] 10.1 Implement cache root initialization with `SKILL_HOME` env var support (via `std::env`) and `dirs::home_dir()` fallback
- [x] 10.2 Implement artifact storage: copy skill package to `cache/artifacts/{hash}/`, compute SHA-256 identity hash via `sha2` crate
- [x] 10.3 Implement metadata index: read/write `metadata/index.json` via serde, add/remove entries, list installed
- [x] 10.4 Implement activation state: read/write `activations/{runtime}.json`, add/remove activation records, track output paths
- [x] 10.5 Implement deactivation: remove activation record and delete generated output files listed in `output_paths`
- [x] 10.6 Implement user config: read `config.json`, deserialize into `UserConfig` struct with default targets and policy
- [x] 10.7 Implement integrity check: verify metadata ↔ artifacts ↔ activations consistency, report orphans
- [x] 10.8 Write tests for: install/uninstall cycle, activate/deactivate cycle, integrity check with orphans, custom SKILL_HOME (using temp dirs)

## 11. CLI Binary Crate

- [x] 11.1 Set up `clap` derive-based CLI with top-level `skill` command, global `--json` flag, and subcommand dispatch
- [x] 11.2 Implement `skill init` — calls `aule_schema::scaffold()`, writes `skill.yaml` template and `content/` directory
- [x] 11.3 Implement `skill validate` — calls schema crate validation, reports errors/warnings to stderr (or JSON to stdout)
- [x] 11.4 Implement `skill build` — validate then call adapter generator, support `--target` and `--output` flags
- [x] 11.5 Implement `skill install` — resolve from local path, call cache manager to store artifact and update metadata
- [x] 11.6 Implement `skill activate` — generate adapter output via adapter crate, update activation state via cache crate, support `--target`
- [x] 11.7 Implement `skill list` — read cache metadata and activation state, display table or JSON
- [x] 11.8 Implement exit codes (0/1/2) and stderr/stdout separation via `std::process::exit`
- [x] 11.9 Write CLI integration tests using `assert_cmd` and `predicates` crates for the full init → validate → build → install → activate → list flow

## 12. End-to-End Validation

- [x] 12.1 Run the full flow for `openspec-explore`: create manifest from existing skill → build → install → activate for both claude-code and codex
- [x] 12.2 Verify generated files match existing hand-written files in `.claude/` and `.codex/`
- [x] 12.3 Run the full flow for all 4 OpenSpec skills and verify correctness
- [x] 12.4 Document any protocol schema adjustments discovered during end-to-end testing
