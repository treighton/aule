## 1. Adapter Definition Types

- [ ] 1.1 Define `AdapterDef` enum in `aule-adapter` with variants `Config { id, description, author, protocol, paths, frontmatter, validate }` and `Script { id, description, author, protocol, generate, validate }`
- [ ] 1.2 Define supporting types: `AdapterPaths` (skill template, optional commands config), `AdapterFrontmatter` (extra_fields vec), `CommandConfig` (path template, display_name template, category, tags template)
- [ ] 1.3 Implement `adapter.yaml` parsing with serde — deserialize both config and script variants from YAML
- [ ] 1.4 Add validation: required fields, valid type values, protocol is positive integer, path templates contain required placeholders
- [ ] 1.5 Write unit tests for adapter.yaml parsing — valid config, valid script, missing fields, unknown type, missing placeholders

## 2. Built-in Adapter Migration

- [ ] 2.1 Express claude-code, codex, and pi as `AdapterDef::Config` constants (or lazy statics)
- [ ] 2.2 Remove `RuntimeTarget` struct constructors (`claude_code()`, `codex()`, `pi()`) and replace with `AdapterDef` equivalents
- [ ] 2.3 Remove `PI_EXTRA_FIELDS` constant — pi's extra fields are now in its `AdapterDef::Config.frontmatter.extra_fields`
- [ ] 2.4 Update `generate.rs` to accept `AdapterDef` instead of `RuntimeTarget` — config-based adapters use the built-in pipeline
- [ ] 2.5 Update `append_adapter_extras()` to use the adapter's `extra_fields` list instead of `PI_EXTRA_FIELDS`
- [ ] 2.6 Update `build_claude_command_frontmatter` to use command config from adapter definition (display_name template, category, tags)
- [ ] 2.7 Verify `real_skills_test` produces byte-identical output — no behavioral changes for built-in adapters

## 3. Adapter Registry

- [ ] 3.1 Create `AdapterRegistry` struct with methods: `by_id(&str) -> Option<AdapterDef>`, `all() -> Vec<(AdapterDef, AdapterSource)>`
- [ ] 3.2 Implement built-in adapter loading (the 3 compiled-in configs)
- [ ] 3.3 Implement user-installed adapter scanning — read `~/.skills/adapters/*/adapter.yaml`
- [ ] 3.4 Implement skill-bundled adapter scanning — read `<base_path>/adapters/*/adapter.yaml`
- [ ] 3.5 Implement precedence: user-installed > skill-bundled > built-in, with deduplication by id
- [ ] 3.6 Update `resolve_targets()` and `resolve_targets_v2()` to use registry lookup instead of `RuntimeTarget::by_id()`
- [ ] 3.7 Update `GenerateOptions` to accept an `AdapterRegistry` (or pass it through `generate_any`)
- [ ] 3.8 Write tests: precedence ordering, deduplication, unknown ID returns None, all() merges sources

## 4. Script Adapter Protocol

- [ ] 4.1 Define input JSON schema types: `ScriptInput { protocol_version, manifest, content, adapter_config, options }`
- [ ] 4.2 Define output JSON schema types: `ScriptOutput { files: Vec<ScriptOutputFile> }`, `ScriptOutputFile { relative_path, content }`
- [ ] 4.3 Implement content resolution — read all skill content, command content, and matched files into the `content` structure
- [ ] 4.4 Implement script execution: serialize input to JSON, spawn subprocess with stdin pipe, capture stdout/stderr, check exit code
- [ ] 4.5 Implement output validation: valid JSON, paths are relative, no path traversal (`..`), valid UTF-8, size limits (10MB per file)
- [ ] 4.6 Implement error handling: structured JSON errors from stderr, fallback to raw stderr display
- [ ] 4.7 Set working directory to adapter directory when executing scripts
- [ ] 4.8 Write tests with a mock script (shell script that echoes expected JSON) — success, validation failure, script crash, path traversal rejection

## 5. Adapter Validation

- [ ] 5.1 Implement validation script execution — same input as generate, parse structured output (valid, errors, warnings)
- [ ] 5.2 Integrate validation into the build pipeline — run before generation, skip generation on errors, display warnings
- [ ] 5.3 Handle validation script crashes — treat as validation failure
- [ ] 5.4 Write tests: validation passes, validation warns, validation fails (generation skipped), validation script crashes

## 6. Protocol Versioning

- [ ] 6.1 Define `MAX_SUPPORTED_PROTOCOL: u32 = 1` constant
- [ ] 6.2 Check adapter protocol version against max during registry loading — error if adapter requires newer protocol
- [ ] 6.3 Default missing `protocol` field to 1
- [ ] 6.4 Include `protocol_version` in script input JSON
- [ ] 6.5 Write tests: supported version passes, unsupported version errors with upgrade message, missing version defaults to 1

## 7. CLI Commands

- [ ] 7.1 Add `skill adapters` subcommand group to `aule-cli`
- [ ] 7.2 Implement `skill adapters list` — display all adapters with id, type, source, description (table and --json)
- [ ] 7.3 Implement `skill adapters add --path <dir>` — validate adapter.yaml, copy to `~/.skills/adapters/<id>/`, handle overwrites with --force
- [ ] 7.4 Implement `skill adapters add --git <url>` — clone, find adapter.yaml, extract id, copy to `~/.skills/adapters/<id>/`
- [ ] 7.5 Implement `skill adapters remove <id>` — delete from `~/.skills/adapters/`, reject built-in removal
- [ ] 7.6 Implement `skill adapters info <id>` — display full adapter details from registry
- [ ] 7.7 Implement `skill adapters test <id>` — run validation, generate against synthetic or --path manifest, verify output integrity
- [ ] 7.8 Update `skill build --target` error message to list available adapters from registry on unknown ID

## 8. Integration Testing

- [ ] 8.1 Create a test config-based adapter (e.g., `test-runtime`) and verify end-to-end generation
- [ ] 8.2 Create a test script-based adapter (shell script) and verify end-to-end generation
- [ ] 8.3 Verify built-in adapters still pass `real_skills_test` with byte-identical output
- [ ] 8.4 Test adapter precedence: install user-level adapter that overrides a built-in, verify it's used
- [ ] 8.5 Test `skill adapters add` / `remove` / `list` / `test` CLI commands end-to-end
