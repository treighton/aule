## Capability: install-infer-flag

Adds `--infer` flag to the existing `skill install` command, enabling installation of repos that don't have a `skill.yaml` by running the inference pipeline first.

## Requirements

### Flag Addition

- Add `--infer` flag to `skill install` command (default: false)
- Flag is compatible with existing source types: local path, git URL
- Flag is ignored for registry sources (`@owner/name`) since those always have manifests

### Behavior

**Without `--infer` (existing behavior, unchanged):**
1. Resolve source → find `skill.yaml` → parse → install
2. No `skill.yaml` found → error: "No skill.yaml found at source"

**With `--infer`:**
1. Resolve source → check for `skill.yaml`
2. `skill.yaml` found → normal install (ignore `--infer`, no warning)
3. `skill.yaml` not found → run inference pipeline:
   - Stage 1 (Discovery): scan known locations
     - Skills found → build manifest, install directly (non-interactive)
   - Stage 2 (LLM Suggest): gather signals, call LLM
     - `can_infer: true` → show suggestions, ask for confirmation, then install
     - `can_infer: false` → error with LLM reasoning
4. The inferred `skill.yaml` is written into the resolved source directory (temp dir for git, in-place for local) before handing off to install logic

### Integration Points

- Reuse the same inference pipeline from `aule-infer` crate
- After inference produces a `ManifestV2`, serialize it as `skill.yaml` into the source directory
- Then call the existing install logic which expects a `skill.yaml` at the source path
- For git sources: the `skill.yaml` is written into the temp cloned directory, installed from there, then temp dir is cleaned up as usual

### Display

```
$ skill install https://github.com/user/cool-tool --infer

Cloning repository...
No skill.yaml found. Running inference...

Scanning known skill locations...
  ✓ Found 1 skill in .claude/skills/

Installing cool-tool v1.0.0...
  ✓ Installed to ~/.cache/aule/cache/artifacts/abc123/
  ✓ skill.yaml generated via inference (discovery)
```

or with LLM:

```
$ skill install https://github.com/user/cool-tool --infer

Cloning repository...
No skill.yaml found. Running inference...

No skills found in known locations.
Analyzing repository...

LLM suggests 1 skill (confidence: 0.85):
  cool-tool — "Generates cool things from templates"

? Install with inferred manifest? [Y/n]
  ✓ Installed to ~/.cache/aule/cache/artifacts/abc123/
  ✓ skill.yaml generated via inference (LLM-assisted)
```

## Acceptance Criteria

- `skill install ./repo --infer` where `skill.yaml` exists → normal install, `--infer` silently ignored
- `skill install ./repo --infer` where no `skill.yaml` but `.claude/skills/` exists → infers and installs without LLM
- `skill install ./repo --infer` where no skills at all → calls LLM, asks for confirmation, then installs
- `skill install ./repo` without `--infer` where no `skill.yaml` → existing error behavior unchanged
- `skill install https://github.com/user/repo --infer` → clones, infers, installs, cleans up temp dir
- `skill install @owner/name --infer` → `--infer` is ignored, normal registry install
