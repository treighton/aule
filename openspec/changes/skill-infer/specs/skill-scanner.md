## Capability: skill-scanner

Deterministic scanners that check known skill locations in a repository and extract existing skills into `DiscoveredSkill` structs.

## Requirements

### Scanner Registry

- Implement scanners for each known skill location format:
  - `ClaudeSkillScanner`: `.claude/skills/**/*.md` — parses YAML frontmatter for name/description
  - `CodexSkillScanner`: `.codex/skills/**/*.md` — same frontmatter parsing
  - `ClaudeCommandScanner`: `.claude/commands/**/*.md` — extracts command name from filename and frontmatter
  - `PluginScanner`: `plugin.json` at repo root — parses plugin manifest for skills, commands, agents
  - `SkillMdScanner`: standalone `SKILL.md` at repo root or in subdirectories — treats as single skill

### Scanner Behavior

- Each scanner receives a `&Path` (repo root) and returns `Result<Vec<DiscoveredSkill>, ScanError>`
- Scanners must not panic on malformed files — return empty vec or skip individual files with warnings
- YAML frontmatter parsing: extract between `---` delimiters, parse as YAML, pull `name` and `description` fields
- If `name` is not in frontmatter, derive from filename (e.g., `SKILL.md` in `foo/` → name `foo`)
- File paths in `DiscoveredSkill` must be relative to repo root

### Orchestration

- `scan_all(repo_root: &Path) -> ScanResult` runs all scanners and merges results
- Deduplication: if multiple scanners find the same skill name, prefer the richer source (plugin.json > skill file > command-only)
- `ScanResult` includes `skills: Vec<DiscoveredSkill>` and `warnings: Vec<String>`

### Edge Cases

- Empty directories (`.claude/skills/` exists but contains no `.md` files) → no results, no error
- Binary files in skill directories → skip silently
- Deeply nested structures → scan recursively but cap depth at 5 levels
- Symlinks → follow (but detect cycles)

## Acceptance Criteria

- Given a repo with `.claude/skills/foo/SKILL.md` containing frontmatter `name: foo`, scanner returns one `DiscoveredSkill` with name `foo` and entrypoint `.claude/skills/foo/SKILL.md`
- Given a repo with `.claude/commands/deploy.md` and `.claude/commands/test.md`, scanner returns discovered commands mapped by name
- Given a repo with `plugin.json` declaring 2 skills and 3 commands, scanner returns matching `DiscoveredSkill` entries
- Given an empty repo, `scan_all` returns empty skills vec with no errors
- Given a repo with malformed YAML frontmatter in one skill file, scanner skips that file and includes a warning
