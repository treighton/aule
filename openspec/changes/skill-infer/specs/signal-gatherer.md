## Capability: signal-gatherer

Collects repo metadata into a structured `InferredSignals` bundle for LLM assessment. Only runs when Stage 1 (skill-scanner) finds no skills.

## Requirements

### Gatherer Types

- `GenericGatherer` (always runs):
  - Reads `README.md` (or `readme.md`, `README.rst`, `README.txt`) — truncates to 8000 tokens (~32k chars)
  - Builds filtered file tree (exclude `.git/`, `node_modules/`, `target/`, `__pycache__/`, `.venv/`, `vendor/`)
  - Detects `LICENSE` / `LICENSE.md` and extracts license type (MIT, Apache-2.0, etc.)
  - Finds executable files in common locations (`bin/`, `scripts/`, `cli/`, repo root `*.sh`)

- `NpmGatherer` (if `package.json` exists):
  - Extracts: name, version, description, author, license, bin entries, main/module, scripts keys
  - Detects runtime version from `engines.node`

- `PythonGatherer` (if `pyproject.toml` or `setup.py` or `setup.cfg` exists):
  - Extracts: name, version, description, author, license, console_scripts entry points
  - Detects runtime version from `requires-python`

- `RustGatherer` (if `Cargo.toml` exists):
  - Extracts: name (package or workspace), version, description, authors, license, binary targets

- `GoGatherer` (if `go.mod` exists):
  - Extracts: module name, Go version
  - Detects `main.go` or `cmd/` directory for executables

### Gatherer Behavior

- Each gatherer receives `&Path` (repo root) and returns `Result<InferredSignals, GatherError>`
- Gatherers run in sequence: generic first, then language-specific
- Language-specific gatherers enrich the generic result (merge, don't replace)
- If multiple language gatherers match (e.g., a repo with both `package.json` and `Cargo.toml`), the primary language is determined by file count heuristic, but all signals merge
- `signal_source` is set to the primary language detected

### Output Constraints

- `readme_content` must be truncated to fit within LLM context limits
- `file_tree` must be filtered and capped at 500 entries
- All paths in `executables` must be relative to repo root
- `InferredSignals` must be serializable to JSON (for LLM prompt construction)

## Acceptance Criteria

- Given an npm repo with `package.json` containing name/version/description/bin, gatherer populates all corresponding fields
- Given a Python repo with `pyproject.toml` and console_scripts, gatherer extracts executable info
- Given a repo with only a README and no package metadata, generic gatherer produces signals with readme_content and file_tree
- Given a repo with 10,000 files, file_tree is capped at 500 entries
- Given a README over 32k characters, readme_content is truncated
