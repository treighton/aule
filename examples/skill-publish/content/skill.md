# Skill Publish

Publish a validated skill to the registry so others can discover and install it.

## Instructions

1. **Check authentication.** Run:

```bash
skill auth status
```

If the user is not authenticated, instruct them to log in first:

```bash
skill login
```

Do not proceed until authentication is confirmed.

2. **Ask for the skill path.** Default to the current directory if not provided.

3. **Run validation** before publishing to catch issues early:

```bash
skill validate --path <path>
```

If validation fails, display the errors and stop. The user must fix them before publishing. Refer them to the `skill-validate` guide for detailed fix suggestions.

4. **Publish the skill:**

```bash
skill publish --path <path>
```

5. **Handle the result:**

### Success

Display the published skill URL and version:

> Published `<name>@<version>` to the registry.
> View it at: https://registry.aule.dev/skills/<name>

Suggest next steps:
- Share the URL with collaborators.
- Install it in another project with `skill install <name>`.
- Bump the version in `skill.yaml` before publishing updates.

### Error Cases

- **Version already exists**: The registry rejects duplicate versions. Bump the version in `skill.yaml` and try again.
- **Network error**: Check internet connectivity and retry.
- **Permission denied**: Confirm the authenticated user owns the skill namespace, or request access.
- **Validation errors at publish time**: The registry runs its own validation. Fix any reported issues and retry.

## Error Handling

- If the `skill` binary is not found, suggest installing it with `cargo install --path crates/aule-cli`.
- If the registry is unreachable, suggest checking https://status.aule.dev for outages.
