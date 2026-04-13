## Context

The `skills/` directory contains 4 OpenSpec skill packages used as both format examples and golden-file test fixtures for the adapter generator. The adapter test (`real_skills_test.rs`) generates SKILL.md files from these sources and compares against checked-in adapter output in `.claude/skills/` and `.codex/skills/`.

We're replacing these with 6 CLI-focused skills that dogfood the `skill` binary, demonstrate the full manifest schema surface area, and introduce the first use of `deps.skills` (skill-to-skill dependencies).

The adapter generator's frontmatter output is limited to `name`, `description`, `license`, `compatibility`, and `metadata`. Manifest fields like `identity`, `tags`, `extensions`, `contract.errors`, and `contract.behavior` are parsed by the schema crate but not emitted in adapter output. This is intentional — the examples prove schema parsing works; extending adapter output is a separate change.

## Goals / Non-Goals

**Goals:**
- Replace OpenSpec example skills with CLI-focused skills that demonstrate the Aulë ecosystem itself
- Cover the full `skill.yaml` manifest surface area across the 6 examples
- Demonstrate skill-to-skill dependencies via `skill-develop` composing 4 simple wrappers
- Demonstrate a complex autonomous workflow with configurable permission gates in `skill-scout`
- Serve as golden-file test fixtures for the adapter generator
- Update all documentation references from `skills/` to `examples/`

**Non-Goals:**
- Extending the adapter generator to emit additional frontmatter fields (`identity`, `tags`, `extensions`, etc.)
- Adding runtime behavior for `deps.skills` resolution (the dependency is declared in the manifest but the resolver doesn't act on it yet)
- Changing the OpenSpec skills themselves — they continue to live in the superpowers plugin, unaffected

## Decisions

### 1. Directory name: `examples/`

**Decision:** Rename `skills/` to `examples/`.

**Rationale:** `skills/` is ambiguous — it looks like production skills that ship with the tool. `examples/` clearly signals these are reference implementations. Alternatives considered: `sample-skills/`, `templates/`. `examples/` is the most conventional (Go, Rust ecosystem norms).

### 2. Six skills in three tiers of complexity

**Decision:** Create a graduated set of examples:

| Tier | Skills | Purpose |
|------|--------|---------|
| Simple | `skill-init`, `skill-validate`, `skill-build`, `skill-publish` | Thin CLI wrappers. Minimal manifests. Show the simplest possible skill. |
| Composer | `skill-develop` | Depends on the 4 simple skills via `deps.skills`. Orchestrates a research → plan → implement → validate loop. First example of skill composition. |
| Complex | `skill-scout` | Full manifest surface area. Autonomous consumer workflow with configurable permission gates. |

**Rationale:** This covers both personas (publisher and consumer), shows the complexity spectrum, and ensures test fixtures exercise different feature combinations. Alternative: fewer, more complex skills. Rejected because the simple wrappers serve a distinct purpose as minimal examples and as dependency targets for `skill-develop`.

### 3. `skill-scout` as the kitchen-sink fixture

**Decision:** `skill-scout` uses every manifest field the schema supports:

```yaml
# Fields exercised by skill-scout
identity: "skills.aule.dev/skill-scout"
contract:
  errors:
    - code: "NO_RESULTS"
      description: "No skills found matching the query"
    - code: "INSTALL_FAILED"
      description: "Skill installation failed"
  behavior:
    timeout_ms: 120000
metadata:
  tags: [consumer, discovery, autonomous, cli]
  homepage: "https://github.com/treightonmauldin/aule"
  repository: "https://github.com/treightonmauldin/aule"
extensions:
  aule:
    gateMode: "supervised"
```

**Rationale:** Having one skill that exercises the full schema proves the schema crate handles all fields correctly, even though the adapter generator doesn't emit them all yet. This creates a natural breadcrumb for the future "extend adapter output" change.

### 4. Configurable autonomy gates in `skill-scout`

**Decision:** The skill content prompts the user to choose a gate mode at the start:

- **Supervised** (4 gates): Ask before search, evaluate, install, and activate+run
- **Autonomous** (1 gate): Search and evaluate silently, then present a single "Found X with permissions Y — install, activate, and run?" prompt

In both modes, the agent always surfaces the skill's permissions before requesting install approval. The gate mode is session-scoped (not persisted).

**Rationale:** This demonstrates how skill content can adapt behavior based on user preference without requiring manifest-level configuration. The permissions are always shown because trust requires transparency — even autonomous mode shouldn't install opaquely.

### 5. `skill-develop` reads docs at runtime

**Decision:** The `skill-develop` skill content instructs the agent to read `docs/authoring-skills.md` at the start of the research phase, rather than embedding the schema reference.

**Rationale:** Embedding the field reference table in the skill content would stale as the schema evolves. Reading the docs at runtime means the skill always has current information. The trade-off is a runtime dependency on the docs file existing, but since `skill-develop` is used within the Aulë repo (or repos that have the docs available), this is acceptable.

### 6. Simple wrappers: determinism levels

**Decision:**
- `skill-init`: `deterministic` — scaffolds a fixed directory structure
- `skill-validate`: `deterministic` — produces pass/fail against fixed rules  
- `skill-build`: `deterministic` — template transform with no variance
- `skill-publish`: `bounded` — involves network call with variable response, but the outcome set is fixed (success, auth error, conflict)

**Rationale:** These are the first examples that use `deterministic` and `bounded` levels (all OpenSpec skills were `probabilistic`). This demonstrates the full determinism vocabulary.

### 7. Test fixture strategy

**Decision:** The `real_skills_test.rs` test changes from 4 functions to 6, one per example skill. The test continues to use the same generate-and-compare pattern. The path changes from `root.join("skills")` to `root.join("examples")`.

**Rationale:** Keeping the same test pattern means minimal Rust code changes. Adding 2 more test functions (net) is low-risk. The new fixtures exercise more manifest features than the old ones.

## Risks / Trade-offs

**Manifest fields without adapter output** — `skill-scout` declares `identity`, `tags`, `extensions`, etc. but the generated SKILL.md won't contain them. Users reading the example might expect to see those fields in the output. → Mitigation: Add a comment in the `skill-scout` skill.yaml noting which fields are "schema-only" pending adapter support.

**`deps.skills` is declaration-only** — `skill-develop` declares dependencies on the 4 simple skills, but the resolver doesn't enforce or auto-install them. → Mitigation: The skill content includes a note that dependencies are advisory in v0. This matches the current state of the codebase.

**OpenSpec adapter output removal** — Deleting `.claude/skills/openspec-*` and `.codex/skills/openspec-*` means those skills no longer auto-load in this repo's Claude Code / Codex sessions. → Mitigation: The OpenSpec skills are installed via the superpowers plugin, not from the repo's adapter output. No functional loss.

**Documentation sweep** — Several files reference `skills/openspec-explore/` in example commands. Missing a reference creates confusing docs. → Mitigation: Grep for `skills/` across all markdown and configuration files before marking the documentation task complete.
