## 1. Directory Rename and Cleanup

- [x] 1.1 Rename `skills/` to `examples/`
- [x] 1.2 Delete OpenSpec adapter output: `.claude/skills/openspec-explore/`, `.claude/skills/openspec-propose/`, `.claude/skills/openspec-apply-change/`, `.claude/skills/openspec-archive-change/` and their `.codex/` equivalents
- [x] 1.3 Remove the 4 OpenSpec skill packages from `examples/` (the former `skills/` contents)

## 2. Simple Wrapper Skills

- [x] 2.1 Create `examples/skill-init/skill.yaml` — minimal manifest, deterministic, no dependencies
- [x] 2.2 Create `examples/skill-init/content/skill.md` — interactive guide for `skill init`
- [x] 2.3 Create `examples/skill-validate/skill.yaml` — minimal manifest, deterministic, no dependencies
- [x] 2.4 Create `examples/skill-validate/content/skill.md` — interactive guide for `skill validate` with error fix suggestions
- [x] 2.5 Create `examples/skill-build/skill.yaml` — minimal manifest, deterministic, `deps.tools: [skill]`
- [x] 2.6 Create `examples/skill-build/content/skill.md` — interactive guide for `skill build` with target selection
- [x] 2.7 Create `examples/skill-publish/skill.yaml` — minimal manifest, bounded, `deps.tools: [skill]`
- [x] 2.8 Create `examples/skill-publish/content/skill.md` — interactive guide for `skill publish` with auth handling

## 3. Composer Skill

- [x] 3.1 Create `examples/skill-develop/skill.yaml` — `deps.skills` referencing all 4 simple wrappers, `deps.tools: [skill]`, probabilistic
- [x] 3.2 Create `examples/skill-develop/content/skill.md` — research → plan → implement → validate loop with runtime schema reading from `docs/authoring-skills.md`

## 4. Consumer Skill

- [x] 4.1 Create `examples/skill-scout/skill.yaml` — full manifest surface area: `identity`, `contract.errors`, `contract.behavior.timeout_ms`, `metadata.tags/homepage/repository`, `extensions`, all permissions
- [x] 4.2 Create `examples/skill-scout/content/skill.md` — autonomous discovery workflow with configurable gate mode (supervised: 4 gates, autonomous: 1 gate)

## 5. Adapter Output Generation

- [x] 5.1 Run `skill build` for all 6 example skills and check in the generated `.claude/skills/` and `.codex/skills/` output

## 6. Test Fixture Update

- [x] 6.1 Update `real_skills_test.rs`: change `root.join("skills")` to `root.join("examples")` in `generate_and_compare`
- [x] 6.2 Replace the 4 OpenSpec test functions with 6 new test functions: `skill_init_matches`, `skill_validate_matches`, `skill_build_matches`, `skill_publish_matches`, `skill_develop_matches`, `skill_scout_matches`
- [x] 6.3 Run `cargo test -p aule-adapter --test real_skills_test` and verify all 6 tests pass

## 7. Documentation Update

- [x] 7.1 Update `CLAUDE.md` — change `skills/` references to `examples/` in repo structure and description
- [x] 7.2 Update `README.md` — change `skills/` path references and example commands to use `examples/`
- [x] 7.3 Update `CONTRIBUTING.md` — change example commands to use `examples/` path
- [x] 7.4 Grep all markdown files for stale `skills/openspec-` references outside of `openspec/changes/` and fix any found

## 8. Final Verification

- [x] 8.1 Run `cargo test` to verify all 96+ tests pass
- [x] 8.2 Run `skill validate` on each of the 6 example skills to confirm valid manifests
