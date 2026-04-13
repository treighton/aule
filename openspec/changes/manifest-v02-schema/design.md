## Context

The v0.1.0 manifest schema (`skill.yaml`) defines a 1:1 mapping between a manifest and a skill. The skill's behavior is entirely expressed as prose in `content/skill.md`, and the contract (inputs, outputs, permissions, determinism) describes the skill's interface. This model works for prompt-based skills but lacks support for skills that ship executable code, require setup steps, or bundle multiple related capabilities.

Through exploration of a reference skill (`api-contract-tester`) that combines deterministic code execution with agentic diagnosis loops, we identified three missing primitives: executable tools, lifecycle hooks, and multi-skill packages. The design draws on GitHub Actions' `action.yml` as prior art for declaring runtimes and entrypoints, adapted for a system where an LLM agent orchestrates tool invocation rather than a CI runner.

Current manifest shape (v0.1.0):
```yaml
schemaVersion: "0.1.0"
name, description, version
content: { skill: "...", commands: { ... } }
contract: { version, inputs, outputs, permissions, determinism, errors, behavior }
adapters: { ... }
metadata: { ... }
dependencies: { ... }
extensions: { ... }
```

## Goals / Non-Goals

**Goals:**
- Support skills that ship executable code alongside prose instructions
- Allow a single package to contain multiple related skills
- Provide lifecycle hooks for setup/teardown during install and activate
- Define typed tool interfaces (JSON Schema input/output) so agents can invoke tools with structured data
- Generate runtime-agnostic wrapper scripts so agents don't need to know the tool's language
- Maintain backward compatibility — v0.1.0 manifests parse without modification
- Build a reference skill (`api-contract-tester`) that exercises all new capabilities

**Non-Goals:**
- Automatic tool composition across packages (future phase — tools are package-scoped for now)
- Runtime sandboxing or isolation of tool execution (covered by existing `process.spawn` permission)
- Streaming or async tool responses (envelope remains synchronous)
- Auto-installation of tool runtime dependencies (e.g., installing Node.js if missing)
- Version resolution across skills within a package (all skills share the package version)

## Decisions

### 1. `contract` → `skills` (map of named skills)

**Decision:** Replace the singular `contract` block with a `skills` map where each key is a skill name and each value contains the skill's entrypoint and full interface definition.

**Rationale:** The word "contract" is jargon-heavy — skill authors think "my skill takes these inputs" not "my contract specifies these inputs." More importantly, making it a map enables multi-skill packages. Each skill gets its own entrypoint (Markdown file), inputs, outputs, permissions, determinism, and errors.

**Alternative considered:** Keep `contract` singular but add a separate `skills` list for multi-skill support. Rejected because it duplicates the interface definition — every skill needs its own permissions and I/O, so the interface belongs inside the skill definition.

```yaml
# v0.2.0
skills:
  contract-tester:
    description: "Generate and run contract tests"
    entrypoint: "content/contract-tester.md"
    version: "1.0.0"
    inputs: { ... }
    outputs: { ... }
    permissions: [...]
    determinism: "bounded"
    errors: [...]
    commands:
      test-api: "content/commands/test-api.md"

  spec-linter:
    description: "Validate an OpenAPI spec"
    entrypoint: "content/spec-linter.md"
    version: "1.0.0"
    inputs: { ... }
    outputs: { ... }
    permissions: ["filesystem.read"]
    determinism: "deterministic"
```

### 2. `content` → `files` (include glob list)

**Decision:** Replace the structured `content` block (`skill`, `commands`) with a flat `files` list of include globs.

**Rationale:** With entrypoints now declared per-skill and per-tool, we don't need `content` to enumerate semantic file types. `files` simply declares what gets bundled — the adapter copies everything matched. Skill markdown, tool scripts, templates, configs — all included uniformly.

**Alternative considered:** Keep `content` for skill/command files and add a separate `logic.files` for executable code. Rejected because it creates two file-declaration mechanisms and forces authors to reason about which bucket a file belongs in.

```yaml
files:
  - "content/**"
  - "logic/**"
```

### 3. Top-level `tools` with per-tool runtime

**Decision:** Tools are declared at the package level (not nested under a skill). Each tool declares its own `using` (runtime), optional `version` constraint, `entrypoint`, and typed `input`/`output` schemas.

**Rationale:** Tools are shared resources — multiple skills in the same package may use the same tool. Per-tool runtime declaration (inspired by GitHub Actions' `using` field) allows a package to mix languages: a Node.js test generator alongside a shell cleanup script. The adapter generates a wrapper script per tool, abstracting the runtime from the agent.

**Alternative considered:** Single `logic.using` at the package level with a single `logic.main` entrypoint using subcommands. Rejected because it forces all tools into one language and one CLI entry, which is less flexible and harder to maintain.

**Alternative considered:** Named entrypoints without JSON Schema types. Rejected because typed input/output enables validation, documentation generation, and future static analysis of skill-tool compatibility.

```yaml
tools:
  generate:
    description: "Generate test harness from an OpenAPI spec"
    using: "node"
    version: ">= 18"
    entrypoint: "logic/tools/generate.ts"
    input: { type: "object", ... }
    output: { type: "object", ... }
```

### 4. Wrapper script generation (adapter)

**Decision:** At `skill build` time, the adapter generates a shell wrapper script for each declared tool. The wrapper invokes the tool's entrypoint with the correct runtime. The adapter also appends a `## Tools` section to each generated SKILL.md documenting available tools, their invocation, and their I/O schemas.

**Rationale:** This is the bridge between the manifest's tool declarations and the agent's actual execution. The agent reads SKILL.md, sees "call `./tools/generate '{...}'`", and uses Bash to execute it. The wrapper handles runtime resolution (`exec node ...`), so the agent never needs to know the tool's language. Works with all existing runtimes today — no changes to Claude Code or Codex needed.

**Alternative considered:** Machine-readable tool blocks (HTML comments or structured metadata) that runtimes parse and register as native tools. More elegant but requires runtime support that doesn't exist. The wrapper approach works now and is forward-compatible — when runtimes add native tool registration, wrappers become the registration target.

**Alternative considered:** Prose-only tool documentation (no wrapper scripts). Rejected because it requires the skill.md author to know the tool's runtime and write correct invocation commands, which couples content to implementation.

Generated structure:
```
.claude/skills/api-testing-suite/
  SKILL.md                    # frontmatter + body + ## Tools section
  tools/
    generate                  # #!/bin/sh → exec node .../generate.ts "$@"
    run-tests                 # #!/bin/sh → exec node .../run-tests.ts "$@"
    report                    # #!/bin/sh → exec node .../report.ts "$@"
  logic/
    tools/generate.ts
    tools/run-tests.ts
    tools/report.ts
    templates/
    hooks/setup.sh
    package.json
```

### 5. Top-level `hooks` for lifecycle events

**Decision:** Hooks are declared at the package level. Three lifecycle events: `onInstall`, `onActivate`, `onUninstall`. Each points to a script that the system (CLI) executes at the corresponding lifecycle moment.

**Rationale:** Skills that ship executable code often need setup (e.g., `npm install` in the logic directory). Rather than relying on prose instructions ("run npm install first") or convention (auto-detect package.json), hooks let the skill author declare exactly what should happen. The system runs it automatically — the agent is not involved.

**v0 simplification:** Hooks are simple string paths to shell scripts. A future version could support object form with per-hook runtime declaration, but for v0 shell scripts cover all practical cases (they can invoke any language's package manager).

```yaml
hooks:
  onInstall: "logic/hooks/setup.sh"
  onActivate: "logic/hooks/verify-runtime.sh"
```

### 6. Backward compatibility via schemaVersion

**Decision:** The parser checks `schemaVersion` and selects the appropriate parsing path. `"0.1.0"` uses the current schema. `"0.2.0"` uses the new schema. Both coexist in the codebase.

**Rationale:** Existing skills (all examples, all published skills) must continue to work. A phased migration is preferable to a flag day. The CLI can offer `skill migrate` to upgrade a v0.1.0 manifest to v0.2.0.

### 7. Commands as a skill property

**Decision:** Commands (slash-commands that invoke a skill) move from `content.commands` to `skills.<name>.commands`. Each skill can declare its own commands.

**Rationale:** A command is a way to invoke a specific skill. In a multi-skill package, commands must be associated with their skill, not the package. This also aligns with how Claude Code plugins work — commands are tied to skills.

## Risks / Trade-offs

**[Wrapper scripts add a layer of indirection]** → Acceptable trade-off. The alternative (agents calling `node ./logic/tools/generate.ts` directly) couples skill content to implementation language. Wrappers are trivial shell scripts — minimal overhead, maximum portability.

**[Multi-skill packages increase manifest complexity]** → Mitigated by keeping single-skill packages as the common case. A package with one skill reads almost identically to v0.1.0. The multi-skill capability is there when needed, not forced.

**[Per-tool runtime declaration could lead to skills requiring many runtimes]** → Mitigated by convention and documentation. Recommend skills use one primary runtime. The flexibility exists for edge cases (shell cleanup script alongside Node.js tools).

**[Hooks execute arbitrary code at install time]** → Same trust model as the skill itself. The user already grants `process.spawn` permission. Hooks run in the same security context. The CLI should display what hooks will run before install (similar to npm's install scripts warning).

**[Schema version branching in the parser]** → Manageable complexity. Two versions is fine. If we reach v0.3.0, consider a more systematic versioning approach (trait-based parsing, schema registry).

## Open Questions

1. **Should tools declare which skills use them?** Currently tools are package-scoped and skills reference them in prose. Adding a `tools: ["generate", "run-tests"]` field to each skill definition would enable static analysis (unused tool detection, permission inference). Defer to a future iteration?

2. **Should the adapter validate that tool entrypoints are syntactically correct for their declared runtime?** e.g., run `node --check` on `.ts` files, `python -m py_compile` on `.py` files. This would catch errors at build time but adds runtime dependencies to the build step.

3. **Hook execution order in multi-skill packages** — `onInstall` runs once for the package, not per skill. `onActivate` runs per skill? Or per package? Need to define the lifecycle more precisely.

4. **Tool I/O transport** — the wrapper scripts need a convention for passing input and receiving output. Options: JSON on stdin/stdout, CLI arguments, temp files. Recommend JSON stdin/stdout as the default, matching the envelope pattern.
