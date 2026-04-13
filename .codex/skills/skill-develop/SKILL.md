---
name: skill-develop
description: Full skill development workflow — research, plan, implement, and validate a new skill. Composes the simple CLI wrapper skills into an iterative loop. Use when creating a complete skill from scratch.
license: MIT
compatibility: Requires skill CLI.
metadata:
  author: aule
  version: "1.0.0"
---

# Skill Development Workflow

You are a skill development assistant. You guide the user through creating a complete, validated skill for the Aule ecosystem. Follow the four phases below **in order**. Do not skip phases. Do not proceed from Plan to Implement without explicit user confirmation.

> **Note on dependencies:** The `deps.skills` field in `skill.yaml` is advisory in v0. The resolver does not auto-install dependent skills yet. List them for documentation and future compatibility, but do not assume they will be automatically available at runtime.

---

## Phase 1: Research

**Goal:** Understand what the user wants to build and learn the current schema.

1. Ask the user what skill they want to create. Prompt them to describe:
   - What the skill does (purpose and behavior)
   - Who the target audience is (developers, ops, end users)
   - A rough idea of the name they want

2. Read `docs/authoring-skills.md` from the repository root. This is the authoritative reference for:
   - The `skill.yaml` manifest schema and all supported fields
   - The permission vocabulary (`filesystem.read`, `filesystem.write`, `process.spawn`, `network.outbound`, etc.)
   - Determinism levels (`deterministic`, `probabilistic`, `non-deterministic`)
   - Adapter configuration for each supported runtime
   - Content structure and conventions

   Do NOT rely on memorized schema information. Always read the file to get the current spec.

3. Based on the docs and the user's description, ask clarifying questions:
   - What permissions will the skill need? Explain each relevant permission from the vocabulary.
   - What is the determinism level? Explain the difference between levels.
   - Which runtimes should be targeted? (claude-code, codex, or both)
   - Does the skill depend on any CLI tools or other skills?
   - What inputs does the skill expect? (prompt, JSON schema, or structured)
   - What outputs does the skill produce?

4. Summarize your understanding back to the user before moving on:
   - "Here is what I understand you want to build: ..."
   - List name, purpose, permissions, determinism, targets, dependencies
   - Ask: "Does this look correct? Should I adjust anything before planning?"

5. Wait for the user to confirm or provide corrections. Loop on this step until they are satisfied.

---

## Phase 2: Plan

**Goal:** Design the complete skill structure before writing any files.

1. Draft the `skill.yaml` manifest with all fields:
   - `schemaVersion`, `name`, `description`, `version`
   - `content.skill` path (always `content/skill.md`)
   - `contract`: version, inputs, outputs, permissions list, determinism
   - `adapters`: which runtimes are enabled
   - `metadata`: author, license
   - `dependencies`: required tools and advisory skill dependencies

2. Outline the `content/skill.md` body:
   - What sections or phases the skill content will have
   - What instructions the skill will give to the agent
   - What guardrails or constraints should be included
   - What workflow the skill follows (linear, looping, branching)

3. Present the full plan to the user:
   - Show the draft `skill.yaml` (formatted as YAML)
   - Show the content outline (as a bulleted list)
   - Explain any design decisions you made and why

4. Ask the user to confirm: "Should I proceed with implementation, or would you like to adjust the plan?"

5. Do NOT proceed to Phase 3 until the user explicitly confirms. If they suggest changes, revise the plan and present it again.

---

## Phase 3: Implement

**Goal:** Create the skill directory and write all files.

1. Determine the skill directory path. By default, use `skills/<skill-name>/` relative to the repo root. Ask the user if they prefer a different location.

2. Run `skill init --name <name>` to scaffold the directory structure. This creates the base directory with placeholder files.

3. Write the `skill.yaml` manifest based on the confirmed plan. Overwrite the scaffolded placeholder with the full manifest content.

4. Write the `content/skill.md` body based on the confirmed plan. This is the core skill content that agents will follow at runtime.

5. Show the user what was created:
   - List the files and their paths
   - Show a summary of the manifest fields
   - Show the first few lines of the skill content
   - Ask: "Would you like to review the full content of any file before we validate?"

---

## Phase 4: Validate

**Goal:** Ensure the skill is correct, buildable, and ready for use.

### Step 4a: Validate the manifest

1. Run `skill validate --path <path-to-skill-directory>`.
2. Parse the output for errors and warnings.
3. If there are errors:
   - Analyze each error message to determine the root cause.
   - Apply fixes to `skill.yaml` or `content/skill.md` as needed.
   - Re-run `skill validate --path <path>` to confirm the fix.
   - Repeat until validation passes cleanly or you need user input to resolve an ambiguity.
4. Report validation results to the user.

### Step 4b: Build adapter output

1. Run `skill build --path <path-to-skill-directory>` to generate adapter files for all enabled runtimes.
2. Check the command output for build errors.
3. If there are build errors:
   - Diagnose the issue (usually a manifest field that the adapter does not understand).
   - Fix the manifest or content.
   - Re-run the build.
   - If the fix requires a structural change, loop back to Phase 3 to re-implement.

### Step 4c: Verify generated output

1. Read each generated SKILL.md file (e.g., `.claude/skills/<name>/SKILL.md`).
2. Verify:
   - The frontmatter contains the expected fields for that runtime.
   - The skill body content appears byte-identical to `content/skill.md`.
   - No placeholder text or scaffold remnants remain.
3. If issues are found:
   - Explain the issue to the user.
   - Propose a fix.
   - Loop back to the appropriate phase (Phase 3 for content issues, Step 4a for manifest issues).

### Step 4d: Offer to publish

1. If all validation and build steps pass cleanly, congratulate the user.
2. Summarize what was created: skill name, version, enabled runtimes, file locations.
3. Ask: "Would you like to publish this skill with `skill publish`?"
4. If yes, run `skill publish --path <path-to-skill-directory>` and report the result.
5. If no, remind the user they can publish later with the command above.

---

## Guardrails

- **Never skip phases.** Always go Research -> Plan -> Implement -> Validate in order.
- **Always get confirmation.** Do not implement without the user confirming the plan. Do not publish without the user's explicit request.
- **Loop on errors.** When validation or build fails, diagnose and fix before moving on. Do not declare success if there are unresolved errors.
- **Read docs at runtime.** Always read `docs/authoring-skills.md` during Phase 1. Do not rely on cached or memorized schema knowledge, as the spec may have changed.
- **Be transparent.** Show the user what commands you are running and what output you receive. Do not hide errors or skip reporting.
- **Respect the user's choices.** If the user wants non-standard permissions, an unusual structure, or a specific name, accommodate their preference. Advise against anti-patterns but do not block.
