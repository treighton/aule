# Skill Validate

Validate a skill package against the manifest schema and report actionable fixes for any errors.

## Instructions

1. Ask the user for the path to the skill directory. If not provided, default to the current directory (`.`).

2. Run the validate command:

```bash
skill validate --path <path>
```

3. Parse the output and handle each case:

### Validation Passes

If the output indicates no errors, congratulate the user:

> Validation passed. The skill package is well-formed and ready to build. Run `skill build` to generate adapter output.

### Validation Fails

For each reported error, provide a specific fix suggestion:

- **Missing required field** (e.g., `name`, `version`): Tell the user to add the field to `skill.yaml` with an example value.
- **Invalid schemaVersion**: List the supported versions and suggest the latest.
- **Missing content file**: Check that `content/skill.md` exists at the path referenced in the manifest. Suggest creating it.
- **Invalid contract fields**: Explain valid values for `inputs`, `outputs` (either `"prompt"` or a JSON Schema object), `determinism` (`"deterministic"`, `"bounded"`, `"unbounded"`), and `permissions`.
- **Invalid adapter config**: Confirm the adapter name matches a supported runtime (`claude-code`, `codex`).

4. After listing fixes, offer to re-run validation once the user has made corrections.

## Error Handling

- If the path does not exist, inform the user and ask them to check the path.
- If no `skill.yaml` is found at the path, suggest running `skill init` first.
- If the `skill` binary is not found, suggest installing it with `cargo install --path crates/aule-cli`.
