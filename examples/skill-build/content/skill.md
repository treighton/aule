# Skill Build

Build adapter output for a skill, generating runtime-specific SKILL.md files from a skill package.

## Instructions

1. Ask the user for:
   - **Skill path**: Path to the skill directory (default: current directory).
   - **Target runtime**: Which runtime to build for (e.g., `claude-code`, `codex`, or `all`). Default to all enabled adapters in the manifest.

2. Run the build command:

```bash
skill build --path <path> --target <target>
```

If the user chose "all" or did not specify a target, omit the `--target` flag to build all enabled adapters.

3. After the build completes, display the generated file paths. These typically follow the pattern:

```
.claude/skills/<skill-name>/SKILL.md    # Claude Code adapter output
.codex/skills/<skill-name>/SKILL.md     # Codex adapter output
```

4. Read and preview the generated SKILL.md frontmatter for the user. The frontmatter contains runtime-specific metadata mapped from the manifest (name, description, version).

5. Explain what adapter output is for:
   - Adapter files are the runtime-specific form of your skill. Each coding agent reads its own format.
   - The skill body (your content) passes through unchanged. Only the frontmatter is transformed per runtime.
   - These files are what get installed into a project for an agent to discover and use.

## Error Handling

- If the build fails with validation errors, suggest running `skill validate` first to diagnose.
- If a target runtime is not enabled in the manifest, inform the user and suggest enabling it in `skill.yaml` under `adapters`.
- If the `skill` binary is not found, suggest installing it with `cargo install --path crates/aule-cli`.
