# Skill Init

Initialize a new skill package with a scaffolded directory structure and manifest.

## Instructions

1. If the user has not provided a skill name, ask them for one. The name should be lowercase, hyphenated, and descriptive (e.g., `code-review`, `test-runner`).

2. Run the init command:

```bash
skill init --name <name>
```

3. After the command completes, display the scaffolded directory structure:

```
<name>/
  skill.yaml          # Skill manifest — identity, contract, adapters
  content/
    skill.md           # Skill body — the instructions your skill provides
```

4. Explain each generated file:
   - **skill.yaml** is the manifest. It defines the skill's name, version, contract (inputs, outputs, permissions), and which runtimes to target.
   - **content/skill.md** is the skill body. This is where you write the actual instructions, workflows, or guides that the skill provides to an agent.

5. Suggest next steps:
   - Edit `skill.yaml` to set the description, author, and contract details.
   - Write the skill content in `content/skill.md`.
   - Run `skill validate` to check the package for errors.
   - Run `skill build` to generate adapter output for your target runtimes.

## Error Handling

- If `skill init` fails because a directory with that name already exists, inform the user and ask if they want to choose a different name.
- If the `skill` binary is not found, suggest installing it with `cargo install --path crates/aule-cli`.
