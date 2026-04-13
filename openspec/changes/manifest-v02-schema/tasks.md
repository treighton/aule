## 1. Schema Types (aule-schema)

- [x] 1.1 Add v0.2.0 manifest types: `SkillDefinition` struct (entrypoint, description, version, inputs, outputs, permissions, determinism, errors, behavior, commands)
- [x] 1.2 Add `Tool` struct (description, using, version, entrypoint, input, output)
- [x] 1.3 Add `Hooks` struct (onInstall, onActivate, onUninstall — all optional string paths)
- [x] 1.4 Add `ManifestV2` struct with top-level fields: name, description, version, schemaVersion, files, skills, tools, hooks, adapters, metadata, dependencies, extensions
- [x] 1.5 Implement schema version routing in parser — dispatch to v0.1.0 or v0.2.0 parsing based on `schemaVersion` field
- [x] 1.6 Validate v0.2.0 rejects `content` and `contract` fields with clear error messages
- [x] 1.7 Validate tool names are kebab-case
- [x] 1.8 Validate tool `using` field against known runtimes (node, python, shell), warn on unknown
- [x] 1.9 Validate tool entrypoints exist on disk and are covered by `files` globs
- [x] 1.10 Validate hook script paths exist on disk
- [x] 1.11 Validate skill entrypoints exist on disk
- [x] 1.12 Validate tool input/output as valid JSON Schema
- [x] 1.13 Write unit tests for v0.2.0 manifest parsing (valid manifests, missing fields, invalid fields)
- [x] 1.14 Write unit tests confirming v0.1.0 manifests continue to parse unchanged

## 2. Adapter Generator (aule-adapter)

- [x] 2.1 Extend adapter to detect manifest version and branch generation logic
- [x] 2.2 Implement per-skill SKILL.md generation — iterate `skills` map, generate one SKILL.md per skill with frontmatter from skill definition
- [x] 2.3 Implement `files` glob resolution — resolve include patterns and copy matched files into output directory preserving relative paths
- [x] 2.4 Implement wrapper script generation — for each tool, generate a shell shim in `tools/` that invokes the entrypoint with the correct runtime command
- [x] 2.5 Implement `## Tools` documentation section — append tool names, descriptions, invocation examples, and I/O schema summaries to each generated SKILL.md
- [x] 2.6 Implement per-skill command generation with namespace derived from skill name
- [x] 2.7 Ensure wrapper scripts are marked executable (chmod +x equivalent in the generator)
- [x] 2.8 Handle overlapping file globs — deduplicate matched files
- [x] 2.9 Write unit tests for wrapper script content generation (node, python, shell runtimes)
- [x] 2.10 Write unit tests for tool documentation section generation
- [x] 2.11 Verify v0.1.0 adapter output is byte-identical (no regressions)

## 3. Cache & CLI — Hook Execution (aule-cache, aule-cli)

- [x] 3.1 Implement hook execution in `aule-cache` or `aule-cli` — run shell script at a given path with working directory set to package directory
- [x] 3.2 Wire `onInstall` hook into `skill install` — execute after successful installation, report status
- [x] 3.3 Wire `onActivate` hook into `skill activate` — execute after successful activation, report status
- [x] 3.4 Wire `onUninstall` hook into `skill uninstall` — execute before file removal, warn on failure but proceed
- [x] 3.5 Handle hook failure — report stderr, warn user, do not roll back install/activate
- [x] 3.6 Write tests for hook execution (success, failure, missing hook)

## 4. CLI — Build Command Updates (aule-cli)

- [x] 4.1 Update `skill build` to pass v0.2.0 manifests through the new adapter path
- [x] 4.2 Update `skill validate` to support v0.2.0 manifests with the new validation rules
- [x] 4.3 Add `skill migrate` subcommand — convert a v0.1.0 manifest to v0.2.0 format (rename contract→skills, content→files, restructure)
- [x] 4.4 Write tests for `skill build` with v0.2.0 manifests
- [x] 4.5 Write tests for `skill migrate` (round-trip: original v0.1.0 → migrated v0.2.0 → build produces equivalent output)

## 5. Example Skill Package (api-contract-tester)

- [x] 5.1 Create `examples/api-contract-tester/skill.yaml` — v0.2.0 manifest with two skills, three tools, hooks, files
- [x] 5.2 Write `content/contract-tester.md` — three-phase agentic workflow (generate → execute → diagnose loop) referencing tools
- [x] 5.3 Write `content/spec-linter.md` — simpler deterministic skill for spec validation
- [x] 5.4 Implement `logic/tools/generate.ts` — parse OpenAPI spec, generate test file stubs, output JSON
- [x] 5.5 Implement `logic/tools/run-tests.ts` — execute generated tests against a base URL, output structured results
- [x] 5.6 Implement `logic/tools/report.ts` — aggregate test results into summary report
- [x] 5.7 Write `logic/hooks/setup.sh` — run `npm install` in logic directory
- [x] 5.8 Write `logic/hooks/verify-runtime.sh` — check `node --version` meets constraint
- [x] 5.9 Create `logic/package.json` with required dependencies
- [x] 5.10 Run `skill validate` and `skill build` on the example, fix any issues
- [x] 5.11 Commit generated adapter output (`.claude/skills/api-contract-tester/`)

## 6. Real Skills Test Update

- [x] 6.1 Add `api-contract-tester` to `real_skills_test.rs` — verify build output matches committed files
- [x] 6.2 Verify wrapper scripts are present and contain expected content
- [x] 6.3 Verify `## Tools` section is present in generated SKILL.md
- [x] 6.4 Verify included files are copied correctly
- [x] 6.5 Ensure all existing v0.1.0 example tests continue to pass

## 7. Platform TypeScript Parser (platform/)

- [x] 7.1 Update `platform/lib/manifest.ts` to detect schema version and parse v0.2.0 shape
- [x] 7.2 Add TypeScript types for `SkillDefinition`, `Tool`, `Hooks`
- [x] 7.3 Update manifest validation to handle both v0.1.0 and v0.2.0 schemas
- [x] 7.4 Update indexer to extract skill-level metadata from v0.2.0 manifests (iterate skills map)
- [x] 7.5 Write tests for v0.2.0 TypeScript manifest parsing

## 8. JSON Schema Update

- [x] 8.1 Update `manifest.schema.json` to support v0.2.0 fields using conditional schemas (if schemaVersion=0.2.0, then require skills/files; if 0.1.0, require content/contract)
- [x] 8.2 Add JSON Schema definitions for Tool, Hooks, SkillDefinition
- [x] 8.3 Validate the updated schema against all example manifests (v0.1.0 and v0.2.0)
